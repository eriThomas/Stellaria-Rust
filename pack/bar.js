import {
  t,
  closeKeyboardPanels,
  language,
  settings,
  actions,
  analyzeImage,
  applyTheme,
  currentMonitorName,
  dashboardState,
  describeWeather,
  element,
  getOption,
  helperClient,
  hideOnFullscreen,
  icon,
  initTheme,
  isPopupOpen,
  isPrimaryMonitor,
  keys,
  loadWeather,
  monitorAtCursor,
  onShared,
  openLauncher,
  openSettings,
  option,
  setOption,
  smoothValue,
  store,
  toast,
  toggleDashboard,
  toggleDrawer,
  togglePopup,
  windowsPalette,
  zebar,
  requestPopout,
} from './common.js';
import {
  createTaskPin,
  filterTasksForMonitor,
  groupTasks,
  isTaskPopoutKind,
  mergePinnedGroups,
  shouldResetTaskPopout,
} from './src/core/taskbar-model.js';

initTheme();
hideOnFullscreen();
const monitor = await currentMonitorName();

const nodes = {
  logo: document.getElementById('logo'),
  workspaces: document.getElementById('workspaces'),
  tasks: document.getElementById('tasks'),
  activeWindow: document.getElementById('active-window'),
  activeTitle: document.getElementById('active-title'),
  tray: document.getElementById('tray'),
  clock: document.getElementById('clock'),
  hours: document.getElementById('clock-hours'),
  minutes: document.getElementById('clock-minutes'),
  seconds: document.getElementById('clock-seconds'),
  keyboard: document.getElementById('keyboard'),
  volume: document.getElementById('volume'),
  volumeIcon: document.getElementById('volume-icon'),
  network: document.getElementById('network'),
  notifications: document.getElementById('notifications'),
  notificationsIcon: document.getElementById('notifications-icon'),
  notificationsBadge: document.getElementById('notifications-badge'),
  networkIcon: document.getElementById('network-icon'),
  power: document.getElementById('power'),
};

const popupButtons = { launcher: nodes.logo, session: nodes.power };

for (const [name, button] of Object.entries(popupButtons)) {
  button.addEventListener('click', () => togglePopup(name, monitor).catch(error => console.error(`[stellaria] ${name}`, error)));
}

nodes.clock.addEventListener('click', () => toggleDashboard(monitor));

const weatherNodes = {
  button: document.getElementById('weather'),
  icon: document.getElementById('weather-icon'),
  temp: document.getElementById('weather-temp'),
};

weatherNodes.button.addEventListener('click', () => {
  store.set(keys.dashboardTab, { tab: 'weather', at: Date.now() });
  toggleDashboard(monitor);
});

function renderWeather(weather) {
  if (!weather?.current) {
    weatherNodes.button.hidden = true;
    return;
  }
  const now = describeWeather(weather.current.code, weather.current.isDay);
  weatherNodes.button.hidden = false;
  weatherNodes.icon.textContent = now.icon;
  weatherNodes.temp.textContent = `${Math.round(weather.current.temperature)}°`;
  weatherNodes.button.title = t('{0} : {1}, {2} °C', weather.city ?? t('Météo'), now.label, Math.round(weather.current.temperature));
}

async function refreshWeather() {
  try {
    renderWeather(await loadWeather());
  } catch (error) {
    console.error('[stellaria] Météo indisponible', error);
  }
}

renderWeather(store.get(keys.weather));
store.watch(keys.weather, renderWeather, 2000);
setInterval(refreshWeather, 15 * 60 * 1000);
option('weather', true, () => refreshWeather());

const syncActiveButtons = () => {
  for (const [name, button] of Object.entries(popupButtons)) {
    button.classList.toggle('active', isPopupOpen(name));
  }
  nodes.clock.classList.toggle('active', dashboardState(monitor).open);
};
syncActiveButtons();
for (const name of Object.keys(popupButtons)) store.watch(keys.popupState(name), syncActiveButtons, 2000);
store.watch(keys.dashboard(monitor), syncActiveButtons, 2000);

const clockState = { seconds: false, timer: 0 };

function renderClock() {
  clearTimeout(clockState.timer);
  const now = new Date();
  nodes.hours.textContent = String(now.getHours()).padStart(2, '0');
  nodes.minutes.textContent = String(now.getMinutes()).padStart(2, '0');
  nodes.seconds.textContent = String(now.getSeconds()).padStart(2, '0');
  nodes.clock.title = now.toLocaleDateString(settings.locale, { weekday: 'long', day: 'numeric', month: 'long', year: 'numeric' });
  const elapsed = (clockState.seconds ? 0 : now.getSeconds() * 1000) + now.getMilliseconds();
  clockState.timer = setTimeout(renderClock, (clockState.seconds ? 1000 : 60000) - elapsed + 20);
}

option('clockSeconds', false, value => {
  clockState.seconds = Boolean(value);
  nodes.seconds.hidden = !clockState.seconds;
  renderClock();
});

const providers = zebar.createProviderGroup({
  audio: { type: 'audio' },
  network: { type: 'network', refreshInterval: 5000 },
  keyboard: { type: 'keyboard', refreshInterval: 1000 },
  systray: { type: 'systray' },
  media: { type: 'media' },
});

const systemEvents = { since: 0, playback: null, recording: null, layout: null, track: null, trackTimer: 0 };

function systemToast(title, message, symbol) {
  if (systemEvents.since && Date.now() - systemEvents.since > 5000 && getOption('systemToasts', true)) toast(title, { message, icon: symbol });
}

function watchSystemEvents(output) {
  const playback = output.audio?.defaultPlaybackDevice;
  const recording = output.audio?.defaultRecordingDevice;
  const layout = output.keyboard?.layout;
  if (playback) {
    if (systemEvents.playback && playback.deviceId !== systemEvents.playback) systemToast(t('Sortie audio'), playback.name, 'speaker');
    systemEvents.playback = playback.deviceId;
  }
  if (recording) {
    if (systemEvents.recording && recording.deviceId !== systemEvents.recording) systemToast(t('Entrée audio'), recording.name, 'mic');
    systemEvents.recording = recording.deviceId;
  }
  if (layout) {
    if (systemEvents.layout && layout !== systemEvents.layout) systemToast(t('Disposition du clavier'), layout, 'keyboard');
    systemEvents.layout = layout;
  }

  const session = output.media?.currentSession;
  const track = session?.title ? `${session.title}${session.artist ? ` — ${session.artist}` : ''}` : null;
  if (session?.isPlaying && track && track !== systemEvents.track) {
    systemEvents.track = track;
    clearTimeout(systemEvents.trackTimer);
    systemEvents.trackTimer = setTimeout(() => {
      if (systemEvents.track === track) systemToast('En lecture', track, 'music_note');
    }, 1500);
  }
}

function volumeIcon(device) {
  if (!device) return 'volume_off';
  if (device.isMuted || device.volume === 0) return 'volume_off';
  if (device.volume < 34) return 'volume_mute';
  if (device.volume < 67) return 'volume_down';
  return 'volume_up';
}

function networkIcon(network) {
  const type = network?.defaultInterface?.type;
  if (type === 'ethernet') return 'lan';
  if (type === 'proprietary_virtual') return 'vpn_lock';
  if (type === 'wifi') {
    const strength = network.defaultGateway?.signalStrength ?? 0;
    if (strength >= 80) return 'signal_wifi_4_bar';
    if (strength >= 60) return 'network_wifi_3_bar';
    if (strength >= 40) return 'network_wifi_2_bar';
    if (strength >= 20) return 'network_wifi_1_bar';
    return 'signal_wifi_0_bar';
  }
  return 'signal_wifi_off';
}

const trayNodes = new Map();

function trayCall(method, id) {
  Promise.resolve(providers.outputMap.systray?.[method](id)).catch(() => {});
}

function renderTray(systray) {
  const icons = systray?.icons ?? [];
  nodes.tray.hidden = icons.length === 0;
  const seen = new Set();

  icons.forEach((trayIcon, index) => {
    seen.add(trayIcon.id);
    let entry = trayNodes.get(trayIcon.id);
    if (!entry) {
      const image = element('img', { alt: '' });
      const button = element('button', {
        class: 'tray-item state-layer',
        onClick: () => trayCall('onLeftClick', trayIcon.id),
        onContextmenu: event => {
          event.preventDefault();
          trayCall('onRightClick', trayIcon.id);
        },
        onAuxclick: event => {
          if (event.button === 1) trayCall('onMiddleClick', trayIcon.id);
        },
        onMouseenter: () => trayCall('onHoverEnter', trayIcon.id),
        onMouseleave: () => trayCall('onHoverLeave', trayIcon.id),
      }, image);
      entry = { button, image, hash: null };
      trayNodes.set(trayIcon.id, entry);
    }
    if (entry.hash !== trayIcon.iconHash) {
      entry.hash = trayIcon.iconHash;
      entry.image.src = trayIcon.iconUrl;
    }
    entry.button.title = trayIcon.tooltip ?? '';
    if (nodes.tray.children[index] !== entry.button) nodes.tray.insertBefore(entry.button, nodes.tray.children[index] ?? null);
  });

  for (const [id, entry] of trayNodes) {
    if (seen.has(id)) continue;
    entry.button.remove();
    trayNodes.delete(id);
  }
  paintTrayOverflow(icons.length);
}

const trayVisibleCount = 4;
let trayExpanded = false;
let trayCollapse = true;
const trayToggle = element('button', {
  class: 'tray-item tray-toggle state-layer',
  onClick: () => {
    trayExpanded = !trayExpanded;
    paintTrayOverflow(trayNodes.size);
  },
}, icon('expand_more'));

function paintTrayOverflow(count) {
  const hiddenCount = trayCollapse && !trayExpanded ? Math.max(0, count - trayVisibleCount) : 0;
  [...nodes.tray.querySelectorAll('.tray-item:not(.tray-toggle)')].forEach((button, index) => button.classList.toggle('collapsed', trayCollapse && !trayExpanded && index >= trayVisibleCount));
  const needed = trayCollapse && count > trayVisibleCount;
  if (!needed) {
    trayToggle.remove();
    return;
  }
  nodes.tray.append(trayToggle);
  trayToggle.querySelector('.icon').textContent = trayExpanded ? 'expand_less' : 'expand_more';
  trayToggle.title = trayExpanded ? t('Réduire la zone de notification') : t('{0} de plus', hiddenCount);
}

option('trayCollapse', true, value => {
  trayCollapse = Boolean(value);
  trayExpanded = false;
  paintTrayOverflow(trayNodes.size);
});

function render(output) {
  const device = output.audio?.defaultPlaybackDevice;
  nodes.volumeIcon.textContent = volumeIcon(device);
  nodes.volume.classList.toggle('muted', Boolean(device?.isMuted));
  nodes.volume.title = device ? `${device.name} — ${device.isMuted ? t('muet') : t('{0} %', Math.round(device.volume))}` : t('Aucune sortie audio');

  nodes.networkIcon.textContent = networkIcon(output.network);
  const gateway = output.network?.defaultGateway;
  nodes.network.title = gateway?.ssid ? t('Wi-Fi : {0}', gateway.ssid) : t(output.network?.defaultInterface ? 'Ethernet connecté' : 'Hors ligne');

  const layout = output.keyboard?.layout ?? '';
  nodes.keyboard.textContent = (layout.split('-').pop() || layout || '--').slice(0, 2).toUpperCase();
  nodes.keyboard.title = layout ? t('Disposition : {0}', layout) : t('Disposition du clavier');

  renderTray(output.systray);
  watchSystemEvents(output);
}

render(providers.outputMap);
providers.onOutput(render);
providers.onError(errors => console.error('[stellaria] providers', JSON.stringify(errors)));

nodes.volume.addEventListener('click', () => {
  const audio = providers.outputMap.audio;
  const device = audio?.defaultPlaybackDevice;
  if (device) audio.setMute(!device.isMuted);
});

nodes.volume.addEventListener('contextmenu', event => {
  event.preventDefault();
  openSettings('sound');
});

nodes.volume.addEventListener('wheel', event => {
  event.preventDefault();
  const audio = providers.outputMap.audio;
  const device = audio?.defaultPlaybackDevice;
  if (!device) return;
  const step = event.deltaY < 0 ? 2 : -2;
  audio.setVolume(Math.min(100, Math.max(0, Math.round(device.volume) + step)));
}, { passive: false });

nodes.network.addEventListener('click', () => openSettings('network-status'));

const popoutTargets = [
  [nodes.activeWindow, 'window'],
  [nodes.volume, 'audio'],
  [nodes.network, 'network'],
  [nodes.keyboard, 'keyboard'],
  [nodes.notifications, 'notifications'],
];

for (const [node, kind] of popoutTargets) {
  node.addEventListener('mouseenter', () => {
    if (!getOption('hoverPopouts', true)) return;
    const rect = node.getBoundingClientRect();
    requestPopout(monitor, {
      kind,
      open: true,
      top: window.screenY + rect.top,
      bottom: window.screenY + rect.bottom,
      at: Date.now(),
    });
  });
}
nodes.keyboard.addEventListener('click', () => openSettings('regionlanguage'));

const notificationKeys = { dismissed: 'stellaria:notifications:dismissed', seenAt: 'stellaria:notifications:seen-at' };
function renderNotificationBadge() {
  const items = helperClient.state('notifications')?.items ?? [];
  const dismissed = new Set(store.get(notificationKeys.dismissed, []));
  const seenAt = store.get(notificationKeys.seenAt, 0);
  const visible = items.filter(item => !dismissed.has(item.id));
  const unread = visible.filter(item => item.time > seenAt).length;
  nodes.notificationsBadge.hidden = unread === 0;
  nodes.notificationsBadge.textContent = unread > 9 ? '9+' : String(unread);
  nodes.notificationsIcon.textContent = unread ? 'notifications_unread' : visible.length ? 'notifications' : 'notifications_none';
  nodes.notifications.title = unread ? t('Notifications ({0} non lues)', unread) : t('Notifications');
}
helperClient.onState('notifications', renderNotificationBadge);
store.watch(notificationKeys.dismissed, renderNotificationBadge, 2000);
store.watch(notificationKeys.seenAt, renderNotificationBadge, 2000);

const desktopIndicator = element('span', { class: 'workspace-indicator' });
const indicatorEdges = { start: 0, end: 0 };
const paintIndicator = () => {
  desktopIndicator.style.top = `${indicatorEdges.start}px`;
  desktopIndicator.style.height = `${Math.max(0, indicatorEdges.end - indicatorEdges.start)}px`;
};
const indicatorStart = smoothValue(value => { indicatorEdges.start = value; paintIndicator(); }, { duration: 200 });
const indicatorEnd = smoothValue(value => { indicatorEdges.end = value; paintIndicator(); }, { duration: 200 });
let indicatorPlaced = false;

function placeIndicator(target) {
  const top = target.offsetTop - 4;
  const bottom = target.offsetTop + target.offsetHeight + 4;
  if (!indicatorPlaced) {
    indicatorPlaced = true;
    indicatorStart.set(NaN);
    indicatorEnd.set(NaN);
    indicatorEdges.start = top;
    indicatorEdges.end = bottom;
    paintIndicator();
    indicatorStart.set(top);
    indicatorEnd.set(bottom);
    return;
  }
  const goingUp = top < indicatorEdges.start;
  indicatorStart.duration = goingUp ? 200 : 450;
  indicatorEnd.duration = goingUp ? 450 : 200;
  indicatorStart.set(top);
  indicatorEnd.set(bottom);
}

function renderDesktops(desktops) {
  const items = desktops?.items ?? [];
  nodes.workspaces.hidden = items.length === 0;
  const buttons = [...nodes.workspaces.querySelectorAll('.workspace')];
  if (buttons.length !== items.length) {
    nodes.workspaces.replaceChildren(desktopIndicator, ...items.map((_, index) => element('button', {
      class: 'workspace state-layer',
      onClick: () => switchDesktop(index),
    }, element('span', { class: 'workspace-dot' }))));
  }
  const current = [...nodes.workspaces.querySelectorAll('.workspace')];
  current.forEach((button, index) => {
    button.classList.toggle('focused', index === desktops.current);
    button.title = items[index].name ?? t('Bureau {0}', index + 1);
  });
  if (current[desktops.current]) requestAnimationFrame(() => placeIndicator(current[desktops.current]));
}

function switchDesktop(index) {
  helperClient.request('desktops.switch', { index }).catch(error => console.error('[stellaria] Bureau virtuel', error));
}

helperClient.onState('desktops', renderDesktops);

nodes.workspaces.addEventListener('wheel', event => {
  event.preventDefault();
  const desktops = helperClient.state('desktops');
  if (!desktops) return;
  const next = desktops.current + (event.deltaY < 0 ? -1 : 1);
  if (next >= 0 && next < desktops.items.length) switchDesktop(next);
}, { passive: false });

function showActiveWindow(title, process, iconData) {
  const label = title?.trim() || process || '';
  nodes.activeWindow.hidden = !label;
  nodes.activeTitle.textContent = label;
  nodes.activeWindow.title = process ? `${label} (${process})` : label;
  const current = nodes.activeWindow.querySelector('.active-icon');
  if (iconData) {
    const source = `data:image/png;base64,${iconData}`;
    if (current?.tagName === 'IMG' && current.src === source) return;
    current.replaceWith(element('img', { class: 'active-icon', src: source, alt: '' }));
  } else if (current?.tagName === 'IMG') {
    current.replaceWith(element('span', { class: 'icon active-icon', text: 'select_window' }));
  }
}

function renderFocus(focus) {
  const entry = focus?.byMonitor?.[monitor];
  showActiveWindow(entry?.title, entry?.process, entry?.icon);
}

renderFocus(store.get(keys.focus));
store.watch(keys.focus, renderFocus, 300);
option('showActiveWindow', false, value => { nodes.activeWindow.hidden = !value; });

/* STELLARIA CUSTOMIZER TASK GROUPING START */
let taskMenuOpen = false;
let lastIdleSeconds = 0;

function resetTaskPopoutState() {
  taskMenuOpen = false;
  const request = store.get(keys.popout(monitor));
  if (request?.open && isTaskPopoutKind(request.kind)) {
    store.set(keys.popout(monitor), { ...request, open: false, at: Date.now() });
  }
}

window.addEventListener('pageshow', resetTaskPopoutState);
document.addEventListener('visibilitychange', () => {
  if (!document.hidden) resetTaskPopoutState();
});

function activateTaskWithoutMinimizing(task) {
  if (task?.active && !getOption('minimizeActiveTaskOnClick', false)) return Promise.resolve();
  return actions.activateTask(task.id);
}

function pinFromTask(task, icons) {
  return createTaskPin(task, icons, store.get(keys.apps)?.items ?? [], t('Application'));
}

async function launchPinnedTask(pin) {
  if (pin?.appId) {
    try {
      await helperClient.request('apps.launch', { id: pin.appId });
      return;
    } catch (error) {
      console.warn('[stellaria] Unable to launch pinned task directly', error);
    }
  }
  await openLauncher(pin?.process || pin?.title || '', monitor);
}

function openTaskGroup(group, node) {
  if (taskMenuOpen) return;
  const rect = node.getBoundingClientRect();
  requestPopout(monitor, {
    kind: 'taskGroup',
    open: true,
    tasks: group.tasks,
    top: window.screenY + rect.top,
    bottom: window.screenY + rect.bottom,
    at: Date.now(),
  });
}

function openTaskMenu(group, node, icons) {
  const task = group.tasks.find(item => item.active) ?? group.tasks[0];
  const pin = group.pin ?? (task ? pinFromTask(task, icons) : null);
  if (!pin) return;
  taskMenuOpen = true;
  const rect = node.getBoundingClientRect();
  requestPopout(monitor, {
    kind: 'taskMenu',
    open: true,
    pin,
    pinned: Boolean(group.pin),
    tasks: group.tasks,
    top: window.screenY + rect.top,
    bottom: window.screenY + rect.bottom,
    at: Date.now(),
  });
}

function taskGroupButton(group, icons) {
  const task = group.tasks.find(item => item.active) ?? group.tasks[0];
  const iconData = (task ? icons[task.icon || task.exe] : null) || group.pin?.icon;
  const count = group.tasks.length;
  const title = task?.process || task?.title || group.pin?.title || t('Application');
  const node = element('button', {
    class: ['task', 'state-layer', group.pin ? 'pinned' : '', count > 1 ? 'grouped' : '', group.tasks.some(item => item.active) ? 'active' : '', count > 0 && group.tasks.every(item => item.minimized) ? 'minimized' : ''].join(' ').trim(),
    title: count > 1 ? `${title} · ${count}` : (task?.title || title),
    onClick: () => count === 0 ? launchPinnedTask(group.pin) : count === 1 ? activateTaskWithoutMinimizing(task) : openTaskGroup(group, node),
    onContextmenu: event => {
      event.preventDefault();
      event.stopPropagation();
      openTaskMenu(group, node, icons);
    },
    onAuxClick: event => {
      if (event.button === 1 && task) actions.closeTask(task.id);
    },
    onMouseEnter: () => {
      if (count > 0 && getOption('taskPreviews', true) && getOption('hoverPopouts', true)) openTaskGroup(group, node);
    },
  }, [
    iconData ? element('img', { src: `data:image/png;base64,${iconData}`, alt: '' }) : icon('select_window'),
    count > 1 ? element('span', { class: 'task-count', text: String(count) }) : null,
  ]);
  return node;
}

function renderTasks() {
  const allTasks = store.get(keys.tasks, []) ?? [];
  const tasks = filterTasksForMonitor(allTasks, monitor, getOption('showTasksFromAllMonitors', true));
  const icons = store.get(keys.taskIcons, {}) ?? {};
  const groups = mergePinnedGroups(
    groupTasks(tasks, getOption('groupTasks', true)),
    store.get(keys.pinnedTasks, []) ?? [],
  );
  nodes.tasks.hidden = groups.length === 0;
  nodes.tasks.replaceChildren(...groups.map(group => taskGroupButton(group, icons)));
}

renderTasks();
store.watch(keys.tasks, renderTasks, 150);
store.watch(keys.pinnedTasks, renderTasks, 150);
store.watch(keys.popout(monitor), request => {
  taskMenuOpen = request?.kind === 'taskMenu' && request?.open === true;
}, 150);
for (const name of ['showTasksFromAllMonitors', 'groupTasks', 'taskPreviews', 'minimizeActiveTaskOnClick']) {
  option(name, name !== 'minimizeActiveTaskOnClick', renderTasks);
}
/* STELLARIA CUSTOMIZER TASK GROUPING END */

function iconFor(exe) {
  return exe ? (store.get(keys.taskIcons, {}) ?? {})[exe] : undefined;
}

function publishFocus(focus) {
  if (!focus) return;
  const byMonitor = {};
  for (const [device, entry] of Object.entries(focus.monitors ?? {})) {
    byMonitor[device] = { title: entry.title, process: entry.process, icon: iconFor(entry.icon || entry.exe) };
  }
  const value = { byMonitor, active: focus.active };
  store.set(keys.focus, value);
  renderFocus(value);
}

async function applyWallpaperSample(wallpaper) {
  if (!wallpaper?.sample) return;
  const { source, achromatic } = await analyzeImage(wallpaper.sample);
  store.set('stellaria:wallpaper', wallpaper.path);
  store.set(keys.wallpaperSample, wallpaper.sample);
  if (store.get(keys.source) !== source || store.get(keys.achromatic, false) !== achromatic) {
    store.set(keys.source, source);
    store.set(keys.achromatic, achromatic);
    applyTheme();
  }
}

async function runHotkey(name, target) {
  target ||= await monitorAtCursor();
  const actions = {
    wallpapers: () => openLauncher(t('>fond '), target),
    launcher: () => openLauncher('', target),
    clipboard: () => openLauncher('>clip ', target),
    emoji: () => openLauncher('>emoji ', target),
    shortcuts: () => openLauncher(t('>raccourcis '), target),
    dashboard: () => toggleDashboard(target),
    session: () => togglePopup('session', target),
    utilities: () => toggleDrawer('utilities', target),
    settings: () => togglePopup('settings', target),
    escape: () => closeKeyboardPanels(),
  };
  await actions[name]?.();
}

function relayHelperState() {
  let locks = null;

  helperClient.onConnection(connected => {
    if (connected) return;
    // Tasks are transient. Never keep dead windows from a previous session when
    // the helper is unavailable; pinned applications live in a separate key.
    store.set(keys.tasks, []);
    renderTasks();
  });

  helperClient.onState('icons', icons => {
    store.set(keys.taskIcons, icons ?? {});
    renderTasks();
    publishFocus(helperClient.state('focus'));
  });
  helperClient.onState('focus', publishFocus);
  helperClient.onState('tasks', tasks => {
    store.set(keys.tasks, tasks?.items ?? []);
    renderTasks();
  });
  helperClient.onState('idle', idle => {
    const seconds = idle?.seconds ?? 0;
    store.set(keys.idle, { seconds, at: Date.now() });
    const threshold = Math.max(1, Number(getOption('idleMinutes', 10)) || 10) * 60 - 15;
    if (shouldResetTaskPopout(lastIdleSeconds, seconds, threshold)) resetTaskPopoutState();
    lastIdleSeconds = seconds;
    const running = Date.now() - store.get('stellaria:idle:running', 0) < 3000;
    if (getOption('idleScreen', true) && seconds >= threshold && !running) {
      store.set('stellaria:idle:running', Date.now());
      zebar.startWidgetPreset('idle', 'default').catch(error => console.error('[stellaria] Écran de veille', error));
    }
  });
  helperClient.onState('keyboard', keyboard => {
    if (locks) {
      if (keyboard.capsLock !== locks.capsLock) systemToast(t(keyboard.capsLock ? 'Verr. Maj activé' : 'Verr. Maj désactivé'), '', 'keyboard_capslock');
      if (keyboard.numLock !== locks.numLock) systemToast(t(keyboard.numLock ? 'Verr. Num activé' : 'Verr. Num désactivé'), '', 'dialpad');
    }
    locks = { capsLock: keyboard.capsLock, numLock: keyboard.numLock };
  });
  helperClient.onState('monitors', monitors => {
    const fullscreen = {};
    const covered = {};
    for (const [device, info] of Object.entries(monitors ?? {})) {
      fullscreen[device] = Boolean(info.fullscreen);
      covered[device] = Boolean(info.covered);
    }
    store.set(keys.fullscreen, { monitors: fullscreen, covered });
  });
  helperClient.onState('wallpaper', wallpaper => {
    applyWallpaperSample(wallpaper).catch(error => console.error('[stellaria] Fond d\'écran', error));
  });
  helperClient.onEvent('hotkey', event => {
    runHotkey(event.name, event.monitor).catch(error => console.error('[stellaria] Raccourci', error));
  });
  helperClient.onEvent('hotkey.unavailable', event => {
    toast(t('Raccourci indisponible'), { message: t('{0} est déjà utilisé par une autre application.', event.shortcut), icon: 'keyboard', kind: 'warning', duration: 6000 });
  });
  helperClient.onEvent('appbars.paused', event => {
    toast(t('Barres Stellaria'), { message: t('Réparations suspendues {0} min.', event.minutes), icon: 'warning', kind: 'warning', duration: 8000 });
  });
}

const switcherRoles = [
  'primary',
  'onPrimary',
  'secondary',
  'tertiary',
  'tertiaryContainer',
  'onTertiaryContainer',
  'surfaceContainerLow',
  'outline',
  'outlineVariant',
  'error',
  'surface',
  'surfaceContainer',
  'surfaceContainerHigh',
  'surfaceContainerHighest',
  'secondaryContainer',
  'onSecondaryContainer',
  'primaryContainer',
  'onPrimaryContainer',
  'onSurface',
  'onSurfaceVariant',
  'shadow',
];
const switcherTheme = { payload: '', confirmed: '', sentAt: 0, timer: 0 };

function syncSwitcherTheme() {
  clearTimeout(switcherTheme.timer);
  switcherTheme.timer = setTimeout(() => {
    const style = getComputedStyle(document.documentElement);
    const theme = {};
    for (const role of switcherRoles) {
      const value = style.getPropertyValue(`--m3-${role.replace(/[A-Z]/g, letter => `-${letter.toLowerCase()}`)}`).trim();
      if (!/^#[0-9a-f]{6}$/i.test(value)) return;
      theme[role] = value;
    }
    theme.transparent = document.documentElement.classList.contains('transparent');
    const payload = JSON.stringify(theme);
    if (payload === switcherTheme.confirmed) return;
    if (payload === switcherTheme.payload && Date.now() - switcherTheme.sentAt < 5000) return;
    switcherTheme.payload = payload;
    switcherTheme.sentAt = Date.now();
    actions.publishSwitcherTheme(theme).catch(error => console.error('[stellaria] Thème du sélecteur', error));
  }, 400);
}

const windowsColors = { sent: '', timer: 0 };

function syncWindowsColors() {
  clearTimeout(windowsColors.timer);
  windowsColors.timer = setTimeout(() => {
    if (!getOption('windowsColors', false)) {
      if (store.get('stellaria:windows-colors:applied', false)) {
        helperClient.request('windows.restoreColors')
          .then(() => store.set('stellaria:windows-colors:applied', false))
          .catch(error => console.error('[stellaria] Couleurs Windows', error));
      }
      windowsColors.sent = '';
      return;
    }
    const primary = getComputedStyle(document.documentElement).getPropertyValue('--m3-primary').trim();
    if (!/^#[0-9a-f]{6}$/i.test(primary)) return;
    const request = {
      palette: windowsPalette(primary),
      dark: document.documentElement.dataset.mode !== 'light',
      titleBars: Boolean(getOption('windowsTitleBars', false)),
    };
    const payload = JSON.stringify(request);
    if (payload === windowsColors.sent) return;
    windowsColors.sent = payload;
    helperClient.request('windows.setColors', request)
      .then(() => store.set('stellaria:windows-colors:applied', true))
      .catch(error => {
        windowsColors.sent = '';
        console.error('[stellaria] Couleurs Windows', error);
      });
  }, 700);
}

const presetOptions = ['tintIcons', 'smartScheme', 'clockSeconds', 'desktopClock', 'island'];
const presetHistoryKey = 'stellaria:preset:history';
const presetDefaults = { tintIcons: true, smartScheme: true, clockSeconds: false, desktopClock: true, island: true };

function currentLook() {
  return {
    scheme: store.get(keys.scheme, null),
    mode: store.get(keys.mode, 'dark'),
    variant: store.get(keys.variant, 'tonalspot'),
    options: Object.fromEntries(presetOptions.map(name => [name, getOption(name, presetDefaults[name])])),
  };
}

function applyLook(look) {
  store.set(keys.scheme, look.scheme ?? null);
  if (look.mode === 'dark' || look.mode === 'light') store.set(keys.mode, look.mode);
  if (look.variant) store.set(keys.variant, look.variant);
  for (const [name, value] of Object.entries(look.options ?? {})) {
    if (presetOptions.includes(name)) setOption(name, Boolean(value));
  }
  applyTheme();
}

function applyPreset(event) {
  const history = store.get(presetHistoryKey, []);
  store.set(presetHistoryKey, [...history, currentLook()].slice(-10));
  const preset = event.preset ?? {};
  applyLook({
    scheme: preset.scheme ?? null,
    mode: preset.mode,
    variant: preset.variant,
    options: preset.options,
  });
  toast(t('Préréglage appliqué'), { message: event.name?.[language] ?? event.name?.fr ?? event.id, icon: 'palette' });
}

function undoPreset() {
  const history = store.get(presetHistoryKey, []);
  const previous = history.pop();
  if (!previous) return;
  store.set(presetHistoryKey, history);
  applyLook(previous);
  toast(t('Apparence précédente rétablie'), { icon: 'undo' });
}

if (await isPrimaryMonitor()) {
  helperClient.onEvent('theme.preset', applyPreset);
  helperClient.onEvent('theme.undo', undoPreset);
  systemEvents.since = Date.now();
  new MutationObserver(syncWindowsColors).observe(document.documentElement, { attributes: true, attributeFilter: ['style', 'data-mode'] });
  option('windowsColors', false, syncWindowsColors);
  option('windowsTitleBars', false, syncWindowsColors);
  helperClient.onConnection(connected => {
    if (connected) {
      windowsColors.sent = '';
      syncWindowsColors();
    }
  });
  relayHelperState();
  onShared('switcher-theme', () => {
    switcherTheme.confirmed = switcherTheme.payload;
  });
  new MutationObserver(syncSwitcherTheme).observe(document.documentElement, { attributes: true, attributeFilter: ['style', 'data-mode', 'class'] });
  setInterval(syncSwitcherTheme, 6000);
  syncSwitcherTheme();
  if (store.get(keys.autoMode, false)) store.set(keys.autoMode, false);
  option('hideTaskbar', true, hidden => actions.setTaskbarHidden(Boolean(hidden)));
  if (!getOption('welcomed', false)) {
    setTimeout(() => {
      setOption('welcomed', true);
      store.set('stellaria:settings:page', { page: 'welcome', at: Date.now() });
      togglePopup('settings').catch(error => console.error('[stellaria] Bienvenue', error));
    }, 4000);
  }
  const configure = (path, value) => helperClient.request('config.set', { path, value })
    .catch(error => console.error(`[stellaria] ${path}`, error));
  option('wallpaperDirectory', '', directory => configure('wallpapers.directory', directory || null));
}
