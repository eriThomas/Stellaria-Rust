export const lazyPanelNames = new Set(['dashboard', 'utilities', 'popout']);

export function canEnsurePanel(name, monitor) {
  return lazyPanelNames.has(name) && Boolean(monitor);
}

export function panelSpawnKey(name, monitor) {
  return `${name}:${monitor}`;
}

export function canSpawnPanel(lastSpawnedAt, now = Date.now(), cooldown = 10000) {
  return now - (lastSpawnedAt ?? 0) >= cooldown;
}

export async function waitForMatchingPong(readPong, id, {
  timeout = 1000,
  interval = 25,
  sleep = delay => new Promise(resolve => setTimeout(resolve, delay)),
} = {}) {
  for (let waited = 0; waited < timeout; waited += interval) {
    await sleep(interval);
    if (readPong() === id) return true;
  }
  return false;
}
