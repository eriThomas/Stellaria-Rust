import * as zebar from '../../vendor/zebar.mjs';
import { currentMonitorName, cursorPosition, hideOnFullscreen, layout, tauriInvoke } from './monitor.js';
import { keys, store } from './store.js';
import { actions } from './helper.js';

const lazyPanels = new Set(['dashboard', 'utilities', 'popout']);
const pendingWindow = 5000;
const spawnedAt = new Map();
let packManifest = null;

const panelKey = (name, monitor, kind) => `stellaria:panel:${name}:${monitor}:${kind}`;

export function answerPanelPings(name, monitor) {
  store.watch(panelKey(name, monitor, 'ping'), ping => {
    if (ping?.id) store.set(panelKey(name, monitor, 'pong'), ping.id);
  }, 100);
}

export function pendingRequest(key) {
  const request = store.get(key);
  return request?.open && Date.now() - request.at < pendingWindow ? request : null;
}

async function presetFor(name) {
  packManifest ??= fetch(new URL('../../zpack.json', import.meta.url)).then(response => response.json());
  const widget = (await packManifest).widgets.find(candidate => candidate.name === name);
  const placement = { ...widget.presets[0] };
  delete placement.name;
  return placement;
}

async function ensurePanel(name, monitor) {
  if (!lazyPanels.has(name) || !monitor) return;
  const id = `${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
  store.set(panelKey(name, monitor, 'ping'), { id, at: Date.now() });
  for (let waited = 0; waited < 1000; waited += 25) {
    await new Promise(resolve => setTimeout(resolve, 25));
    if (store.get(panelKey(name, monitor, 'pong')) === id) return;
  }
  const key = `${name}:${monitor}`;
  if (Date.now() - (spawnedAt.get(key) ?? 0) < 10000) return;
  spawnedAt.set(key, Date.now());
  await zebar.startWidget(name, { ...(await presetFor(name)), monitorSelection: { type: 'name', match: monitor } });
}

function ensureInBackground(name, monitor) {
  ensurePanel(name, monitor).catch(error => console.error(`[stellaria] Chargement de ${name}`, error));
}

export function letClicksThrough() {
  return zebar.currentWidget().tauriWindow.setIgnoreCursorEvents(true).catch(error => console.error('[stellaria] Clics traversants', error));
}

export function drawerKey(name, monitor) {
  return `stellaria:${name}:${monitor}`;
}

export function requestDrawer(name, monitor, open, trigger = 'hover') {
  store.set(drawerKey(name, monitor), { open, trigger, at: Date.now() });
  if (open) ensureInBackground(name, monitor);
}

export function requestPopout(monitor, request) {
  store.set(keys.popout(monitor), request);
  if (request.open) ensureInBackground('popout', monitor);
}

export async function edgeDrawer(name, { panel, side, autoCloseMs = 1600, onOpen, onClose }) {
  const monitorName = await currentMonitorName();
  const widgetWindow = zebar.currentWidget().tauriWindow;
  const key = drawerKey(name, monitorName);
  const state = { open: false, trigger: null, lastInside: 0, entered: false, openedAt: 0, closeTimer: null, suppressed: false };

  const inside = cursor => {
    const rect = panel.getBoundingClientRect();
    const x = window.screenX;
    const y = window.screenY;
    const r = layout.rounding;
    if (side === 'right') {
      return cursor.x >= x + rect.left - r && cursor.x <= x + window.innerWidth + 1 &&
        cursor.y >= y + rect.top - r && cursor.y <= y + rect.bottom + r;
    }
    if (side === 'bottom') {
      return cursor.y >= y + rect.top - r && cursor.y <= y + window.innerHeight + 1 &&
        cursor.x >= x + rect.left - r && cursor.x <= x + rect.right + r;
    }
    return cursor.x >= x + rect.left - r && cursor.x <= x + rect.right + r &&
      cursor.y >= y + rect.top - r && cursor.y <= y + rect.bottom + r;
  };

  const hide = () => {
    if (!state.open) return;
    state.open = false;
    if (state.holdsEscape) {
      state.holdsEscape = false;
      actions.holdEscape(`${name}:${monitorName}`, false);
    }
    document.body.classList.remove('open');
    document.body.classList.add('closing');
    onClose?.();
    store.set(key, { open: false, trigger: state.trigger, at: Date.now() });
    state.closeTimer = setTimeout(() => {
      document.body.classList.remove('closing');
      widgetWindow.setIgnoreCursorEvents(true).catch(() => {});
    }, 380);
  };

  const watch = async () => {
    while (state.open) {
      try {
        const cursor = await cursorPosition();
        if (!state.open) break;
        const now = Date.now();
        if (inside(cursor)) {
          state.lastInside = now;
          state.entered = true;
        } else if (state.trigger === 'hover' || (state.entered && state.trigger !== 'pinned')) {
          if (now - state.lastInside > 150) hide();
        } else if (state.trigger !== 'pinned' && now - state.openedAt > autoCloseMs) {
          hide();
        }
      } catch (error) {
        console.error(`[stellaria] ${name} : curseur indisponible`, error);
      }
      await new Promise(resolve => setTimeout(resolve, 50));
    }
  };

  const show = async (trigger, refresh = false) => {
    clearTimeout(state.closeTimer);
    state.trigger = state.open && state.trigger === 'hover' && trigger !== 'hover' ? 'hover' : trigger;
    state.openedAt = Date.now();
    state.lastInside = Date.now();
    if (state.open) {
      if (refresh) onOpen?.(trigger);
      return;
    }
    state.open = true;
    state.entered = false;
    if (state.trigger === 'pinned') {
      state.holdsEscape = true;
      actions.holdEscape(`${name}:${monitorName}`, true);
    }
    onOpen?.(trigger);
    document.body.classList.remove('closing');
    requestAnimationFrame(() => document.body.classList.add('open'));
    await widgetWindow.setIgnoreCursorEvents(false);
    if (!state.open) {
      await letClicksThrough();
      return;
    }
    watch();
  };

  hideOnFullscreen(value => {
    state.suppressed = value;
    if (value) hide();
  });

  await widgetWindow.setIgnoreCursorEvents(true);
  answerPanelPings(name, monitorName);
  const pending = pendingRequest(key);
  if (!pending) store.set(key, { open: false, at: Date.now() });
  store.watch(key, request => {
    if (request?.open && !state.suppressed) show(request.trigger ?? 'hover');
    else if (request && !request.open && request.trigger === 'pinned') hide();
  }, 60);
  if (pending && !state.suppressed) show(pending.trigger ?? 'hover');

  return {
    monitor: monitorName,
    isOpen: () => state.open,
    isSuppressed: () => state.suppressed,
    show: trigger => (state.suppressed ? null : show(trigger, true)),
    hide,
  };
}

export function closeKeyboardPanels() {
  for (const key of Object.keys(localStorage)) {
    if (!key.startsWith('stellaria:dashboard:') && !key.startsWith('stellaria:utilities:')) continue;
    const value = store.get(key);
    if (!value?.open || value.trigger === 'hover') continue;
    store.set(key, { open: false, trigger: value.trigger, at: Date.now() });
  }
}

export function toggleDrawer(name, monitor) {
  const current = store.get(drawerKey(name, monitor));
  requestDrawer(name, monitor, !current?.open, 'pinned');
}

export function requestDashboard(monitor, open, trigger = 'click') {
  store.set(keys.dashboard(monitor), { open, trigger, at: Date.now() });
  if (open) ensureInBackground('dashboard', monitor);
}

export function dashboardState(monitor) {
  return store.get(keys.dashboard(monitor), { open: false, trigger: 'click', at: 0 });
}

export function toggleDashboard(monitor) {
  const state = dashboardState(monitor);
  requestDashboard(monitor, !(state.open && state.trigger === 'click'), 'click');
}

const popupPlacements = {
  launcher: { anchor: 'bottom_center', offsetX: '19px', offsetY: '0px', width: '740px', height: '640px' },
  session: { anchor: 'center_right', offsetX: '0px', offsetY: '0px', width: '170px', height: '590px' },
  settings: { anchor: 'center', offsetX: '19px', offsetY: '0px', width: '1060px', height: '780px' },
};

async function askResident(name, monitor) {
  const id = `${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
  store.set(keys.popupRequest(name), { id, monitor: monitor ?? 'primary', at: Date.now() });
  for (let waited = 0; waited < 400; waited += 25) {
    await new Promise(resolve => setTimeout(resolve, 25));
    if (store.get(keys.popupAck(name)) === id) return true;
  }
  return false;
}

async function placePopup(name, monitorName) {
  const placement = popupPlacements[name];
  const monitors = await tauriInvoke('plugin:window|available_monitors');
  const monitor = monitors.find(candidate => candidate.name === monitorName);
  if (!placement || !monitor) return;
  const scale = monitor.scaleFactor || 1;
  const width = parseFloat(placement.width) * scale;
  const height = parseFloat(placement.height) * scale;
  const offsetX = parseFloat(placement.offsetX) * scale;
  const { x, y } = monitor.position;
  const { width: screenWidth, height: screenHeight } = monitor.size;
  const positions = {
    bottom_center: [x + (screenWidth - width) / 2 + offsetX, y + screenHeight - height],
    center_right: [x + screenWidth - width, y + (screenHeight - height) / 2],
    center: [x + (screenWidth - width) / 2 + offsetX, y + (screenHeight - height) / 2],
  };
  const [left, top] = positions[placement.anchor] ?? positions.center;
  await tauriInvoke('plugin:window|set_position', { value: { Physical: { x: Math.round(left), y: Math.round(top) } } });
}

async function startPopup(name, monitor) {
  if (await askResident(name, monitor)) return;
  store.set(keys.popupSpawn(name), { monitor: monitor ?? 'primary', at: Date.now() });
  if (!monitor) {
    await zebar.startWidgetPreset(name, 'default');
    return;
  }
  await zebar.startWidget(name, {
    ...popupPlacements[name],
    monitorSelection: { type: 'name', match: monitor },
    dockToEdge: { enabled: false, edge: null, windowMargin: '0px' },
  });
}

export async function openLauncher(query = '', monitor) {
  store.set(keys.launcherQuery, { query, at: Date.now() });
  if (isPopupOpen('launcher')) return;
  await startPopup('launcher', monitor);
}

export function isPopupOpen(name) {
  return store.get(keys.popupState(name), false) === true;
}

export async function togglePopup(name, monitor) {
  if (isPopupOpen(name)) {
    store.set(keys.popupDismiss(name), Date.now());
    return;
  }
  if (Date.now() - store.get(keys.popupClosed(name), 0) < 400) return;
  await startPopup(name, monitor);
}

export function setupPopup(name, { panel, onOpen } = {}) {
  const widget = zebar.currentWidget();
  const window_ = widget.tauriWindow;
  const loadedAt = Date.now();
  const spawn = store.get(keys.popupSpawn(name), null);
  const transient = Boolean(spawn && loadedAt - spawn.at < 8000);
  if (transient) store.set(keys.popupSpawn(name), null);
  let monitorName = null;
  let open = false;
  let openedAt = 0;
  let hideTimer = 0;

  const located = currentMonitorName().then(monitor => {
    monitorName = monitor;
  });

  const show = async () => {
    clearTimeout(hideTimer);
    openedAt = Date.now();
    if (open) {
      onOpen?.();
      return;
    }
    open = true;
    store.set(keys.popupState(name), true);
    document.body.classList.remove('closing');
    await window_.show().catch(() => {});
    await window_.setFocus().catch(() => {});
    requestAnimationFrame(() => requestAnimationFrame(() => {
      document.body.classList.add('open');
      onOpen?.();
    }));
  };

  const close = () => {
    if (!open) return;
    open = false;
    store.set(keys.popupState(name), false);
    store.set(keys.popupClosed(name), Date.now());
    document.body.classList.remove('open');
    document.body.classList.add('closing');
    clearTimeout(hideTimer);
    hideTimer = setTimeout(() => {
      if (open) return;
      document.body.classList.remove('closing');
      if (transient) widget.close();
      else window_.hide().catch(() => {});
    }, 380);
  };

  let placedOn = null;
  store.watch(keys.popupRequest(name), async request => {
    if (transient || !request || request.at <= loadedAt) return;
    await located;
    store.set(keys.popupAck(name), request.id);
    const target = request.monitor === 'primary' ? monitorName : request.monitor;
    if (target !== placedOn) {
      await placePopup(name, target).catch(error => console.error(`[stellaria] Placement de ${name}`, error));
      placedOn = target;
    }
    show();
  }, 150);
  located.then(() => { placedOn = monitorName; });

  store.watch(keys.popupDismiss(name), value => {
    if (value > openedAt) close();
  }, 150);

  window_.onFocusChanged(({ payload: focused }) => {
    if (focused || !open) return;
    if (Date.now() - openedAt < 1000) {
      window_.setFocus().catch(() => {});
      return;
    }
    close();
  });

  window.addEventListener('keydown', event => {
    if (event.key === 'Escape' && open) {
      event.preventDefault();
      close();
    }
  });

  document.addEventListener('mousedown', event => {
    if (open && panel && !panel.contains(event.target)) close();
  });

  if (transient) document.fonts.ready.then(show);
  else {
    store.set(keys.popupState(name), false);
    window_.hide().catch(() => {});
  }

  return { close, show };
}
