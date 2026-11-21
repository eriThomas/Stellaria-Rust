import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import path from 'node:path';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';
import { clampBorderSize, customizerDefaults } from '../../pack/src/core/customizer.js';
import {
  createTaskPin,
  filterTasksForMonitor,
  groupTasks,
  isTaskPopoutKind,
  isTaskPreviewBlocked,
  matchingStartApp,
  mergePinnedGroups,
  normalizedAppToken,
  shouldResetTaskPopout,
  taskGroupKey,
} from '../../pack/src/core/taskbar-model.js';
import {
  canEnsurePanel,
  canSpawnPanel,
  panelSpawnKey,
  waitForMatchingPong,
} from '../../pack/src/core/panel-policy.js';

const root = path.join(path.dirname(fileURLToPath(import.meta.url)), '..', '..');

test('customizer defaults preserve the intended desktop behaviour', () => {
  assert.deepEqual(customizerDefaults, {
    borderSize: 0,
    showActiveWindow: false,
    showTasksFromAllMonitors: true,
    launcherWinKey: false,
    groupTasks: true,
    taskPreviews: true,
    minimizeActiveTaskOnClick: false,
  });
  assert.ok(Object.isFrozen(customizerDefaults));
});

test('border size is numeric and clamped to the supported range', () => {
  assert.equal(clampBorderSize(-4), 0);
  assert.equal(clampBorderSize('12'), 12);
  assert.equal(clampBorderSize(100), 40);
  assert.equal(clampBorderSize('invalid'), 0);
});

test('task keys prefer executable, then process, then window id', () => {
  assert.equal(taskGroupKey({ exe: 'C:\\Apps\\Vivaldi.EXE', process: 'vivaldi', id: '1' }), 'c:\\apps\\vivaldi.exe');
  assert.equal(taskGroupKey({ process: 'Terminal', id: '2' }), 'terminal');
  assert.equal(taskGroupKey({ id: 42 }), '42');
});

test('tasks from the same executable are grouped case-insensitively', () => {
  const groups = groupTasks([
    { id: '1', exe: 'Vivaldi.exe' },
    { id: '2', exe: 'VIVALDI.EXE' },
    { id: '3', exe: 'Code.exe' },
  ]);
  assert.deepEqual(groups.map(group => [group.key, group.tasks.map(task => task.id)]), [
    ['vivaldi.exe', ['1', '2']],
    ['code.exe', ['3']],
  ]);
});

test('task grouping can be disabled without losing individual windows', () => {
  const groups = groupTasks([{ id: 'a', exe: 'same.exe' }, { id: 'b', exe: 'same.exe' }], false);
  assert.deepEqual(groups.map(group => group.key), ['a', 'b']);
  assert.deepEqual(groups.map(group => group.tasks.length), [1, 1]);
});

test('monitor filtering either keeps every task or only the local monitor', () => {
  const tasks = [{ id: 1, monitor: 'DISPLAY1' }, { id: 2, monitor: 'DISPLAY2' }];
  assert.deepEqual(filterTasksForMonitor(tasks, 'DISPLAY1', false).map(task => task.id), [1]);
  assert.deepEqual(filterTasksForMonitor(tasks, 'DISPLAY1', true).map(task => task.id), [1, 2]);
});

test('start app matching tolerates accents, punctuation and exe suffixes', () => {
  assert.equal(normalizedAppToken('Vivaldí Browser.exe'), 'vivaldibrowser');
  const apps = [
    { id: 'fallback', name: 'Vivaldi' },
    { id: 'C:\\Apps\\Vivaldi.exe', name: 'Different label' },
  ];
  assert.equal(matchingStartApp({ exe: 'C:\\Apps\\Vivaldi.exe', process: 'vivaldi' }, apps), apps[1]);
});

test('pins keep a launch target and the current application icon', () => {
  const task = { id: '1', exe: 'C:\\Apps\\FilePilot.exe', process: 'File Pilot', icon: 'filepilot-icon' };
  const pin = createTaskPin(task, { 'filepilot-icon': 'base64-icon' }, [{ id: 'filepilot-app', name: 'File Pilot' }]);
  assert.deepEqual(pin, {
    key: 'c:\\apps\\filepilot.exe',
    title: 'File Pilot',
    process: 'File Pilot',
    exe: 'C:\\Apps\\FilePilot.exe',
    appId: 'filepilot-app',
    icon: 'base64-icon',
  });
});

test('pinned applications stay first, survive closing and are not duplicated', () => {
  const running = [
    { key: 'vivaldi.exe', tasks: [{ id: '1' }] },
    { key: 'code.exe', tasks: [{ id: '2' }] },
  ];
  const pins = [
    { key: 'filepilot.exe', title: 'FilePilot' },
    { key: 'vivaldi.exe', title: 'Vivaldi' },
    { key: 'vivaldi.exe', title: 'duplicate' },
  ];
  const merged = mergePinnedGroups(running, pins);
  assert.deepEqual(merged.map(group => group.key), ['filepilot.exe', 'vivaldi.exe', 'code.exe']);
  assert.deepEqual(merged[0].tasks, []);
  assert.equal(merged[1].tasks[0].id, '1');
  assert.equal(merged[1].pin.title, 'Vivaldi');
});

test('unsafe NanaZip previews fall back to an icon', () => {
  assert.equal(isTaskPreviewBlocked({ process: 'NanaZip' }), true);
  assert.equal(isTaskPreviewBlocked({ exe: 'C:\\Apps\\NANAZIP.Modern.exe' }), true);
  assert.equal(isTaskPreviewBlocked({ process: 'Vivaldi' }), false);
});

test('idle transitions reset task popouts only on enter and resume boundaries', () => {
  assert.equal(shouldResetTaskPopout(100, 585, 585), true);
  assert.equal(shouldResetTaskPopout(600, 0, 585), true);
  assert.equal(shouldResetTaskPopout(600, 500, 585), false);
  assert.equal(shouldResetTaskPopout(100, 200, 585), false);
  assert.equal(isTaskPopoutKind('taskGroup'), true);
  assert.equal(isTaskPopoutKind('taskMenu'), true);
  assert.equal(isTaskPopoutKind('audio'), false);
});

test('every lazy panel, including the primary popout, receives a health check', () => {
  assert.equal(canEnsurePanel('popout', 'DISPLAY1'), true);
  assert.equal(canEnsurePanel('dashboard', 'PRIMARY'), true);
  assert.equal(canEnsurePanel('utilities', 'DISPLAY4'), true);
  assert.equal(canEnsurePanel('launcher', 'DISPLAY1'), false);
  assert.equal(canEnsurePanel('popout', ''), false);
});

test('panel liveness waits for a matching pong and times out cleanly', async () => {
  let reads = 0;
  const alive = await waitForMatchingPong(() => (++reads >= 3 ? 'expected' : 'stale'), 'expected', {
    timeout: 100,
    interval: 25,
    sleep: async () => {},
  });
  assert.equal(alive, true);
  assert.equal(reads, 3);

  const dead = await waitForMatchingPong(() => 'stale', 'expected', {
    timeout: 100,
    interval: 25,
    sleep: async () => {},
  });
  assert.equal(dead, false);
});

test('panel respawning is keyed per monitor and rate limited', () => {
  assert.equal(panelSpawnKey('popout', 'DISPLAY2'), 'popout:DISPLAY2');
  assert.equal(canSpawnPanel(1000, 10999), false);
  assert.equal(canSpawnPanel(1000, 11000), true);
  assert.equal(canSpawnPanel(undefined, 11000), true);
});

test('bar and popout bypass stale WebView caching without disabling other widgets', async () => {
  const manifest = JSON.parse(await readFile(path.join(root, 'pack', 'zpack.json'), 'utf8'));
  const widgets = Object.fromEntries(manifest.widgets.map(widget => [widget.name, widget]));
  assert.equal(widgets.bar.caching.defaultDuration, 0);
  assert.equal(widgets.popout.caching.defaultDuration, 0);
  for (const widget of manifest.widgets.filter(widget => !['bar', 'popout'].includes(widget.name))) {
    assert.ok(widget.caching.defaultDuration > 0, widget.name);
  }
  assert.equal(widgets.bar.presets[0].monitorSelection.type, 'all');
});

test('integrated settings expose every customizer preference', async () => {
  const settings = await readFile(path.join(root, 'pack', 'settings.js'), 'utf8');
  for (const key of Object.keys(customizerDefaults)) {
    assert.match(settings, new RegExp(`(?:['"]${key}['"]|\\.${key}\\b)`), key);
  }
  const schema = await readFile(path.join(root, 'helper', 'src', 'Stellaria.Helper', 'Configuration', 'ConfigSchema.cs'), 'utf8');
  assert.match(schema, /Flag\("launcher\.winKey", false\)/);
});

test('native preview and switcher safety regressions remain guarded', async () => {
  const windowModule = await readFile(path.join(root, 'helper', 'src', 'Stellaria.Helper', 'Modules', 'WindowModule.cs'), 'utf8');
  assert.match(windowModule, /request\.Params\["id"\]/);
  assert.match(windowModule, /TasksModule\.ParseId\(taskId\)/);
  assert.match(windowModule, /IsCaptureBlocked\(window\)/);
  assert.match(windowModule, /Contains\("NanaZip", StringComparison\.OrdinalIgnoreCase\)/);

  const switcher = await readFile(path.join(root, 'pack', 'native', 'StellariaSwitcher.cs'), 'utf8');
  assert.match(switcher, /Commit\(opacity, false\)/);
  assert.match(switcher, /Interval = 20/);
  assert.match(switcher, /InitializeBegin\(\)/);
});
