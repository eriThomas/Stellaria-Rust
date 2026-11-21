export const taskPopoutKinds = new Set(['taskGroup', 'taskMenu']);

export function taskGroupKey(task) {
  return String(task?.exe || task?.process || task?.id || '').toLowerCase();
}

export function normalizedAppToken(value) {
  return String(value || '')
    .replace(/\.exe$/i, '')
    .normalize('NFD')
    .replace(/[\u0300-\u036f]/g, '')
    .replace(/[^a-z0-9]/gi, '')
    .toLowerCase();
}

export function matchingStartApp(task, apps = []) {
  const exe = String(task?.exe || '').toLowerCase();
  const process = normalizedAppToken(task?.process);
  const executable = normalizedAppToken(String(task?.exe || '').split(/[\\/]/).pop());
  let best = null;
  let bestScore = 0;
  for (const app of apps) {
    const id = String(app?.id || '').toLowerCase();
    const name = normalizedAppToken(app?.name);
    let score = id === exe && exe ? 120 : 0;
    if (name && (name === process || name === executable)) score = Math.max(score, 100);
    else if (name && ((process && process.includes(name)) || (executable && executable.includes(name)))) score = Math.max(score, 70);
    if (score > bestScore) {
      best = app;
      bestScore = score;
    }
  }
  return best;
}

export function groupTasks(tasks, enabled = true) {
  if (!enabled) return tasks.map(task => ({ key: String(task.id), tasks: [task] }));
  const grouped = new Map();
  for (const task of tasks) {
    const key = taskGroupKey(task);
    if (!grouped.has(key)) grouped.set(key, { key, tasks: [] });
    grouped.get(key).tasks.push(task);
  }
  return [...grouped.values()];
}

export function filterTasksForMonitor(tasks, monitor, showAll = true) {
  return showAll ? [...tasks] : tasks.filter(task => task.monitor === monitor);
}

export function createTaskPin(task, icons = {}, apps = [], fallbackTitle = 'Application') {
  const app = matchingStartApp(task, apps);
  return {
    key: taskGroupKey(task),
    title: task?.process || task?.title || fallbackTitle,
    process: task?.process || '',
    exe: task?.exe || '',
    appId: app?.id || task?.exe || null,
    icon: icons[task?.icon || task?.exe] || null,
  };
}

export function mergePinnedGroups(groups, pins = []) {
  const remaining = new Map(groups.map(group => [group.key, group]));
  const result = [];
  const seen = new Set();
  for (const pin of pins) {
    if (!pin?.key || seen.has(pin.key)) continue;
    seen.add(pin.key);
    const group = remaining.get(pin.key) ?? { key: pin.key, tasks: [] };
    group.pin = pin;
    remaining.delete(pin.key);
    result.push(group);
  }
  return result.concat([...remaining.values()]);
}

export function isTaskPreviewBlocked(task) {
  return `${task?.process ?? ''} ${task?.exe ?? ''}`.toLowerCase().includes('nanazip');
}

export function shouldResetTaskPopout(previousSeconds, seconds, threshold) {
  const enteredIdle = previousSeconds < threshold && seconds >= threshold;
  const resumedFromIdle = previousSeconds >= threshold && seconds < 5;
  return enteredIdle || resumedFromIdle;
}

export function isTaskPopoutKind(kind) {
  return taskPopoutKinds.has(kind);
}
