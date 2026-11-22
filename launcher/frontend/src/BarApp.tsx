import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { useEffect, useRef, useState } from 'react';
import { Icon } from './components/ExpressiveShape';
import { applyMaterialTheme } from './theme/materialYou';
import type { DashboardSnapshot, MonitorInfo, WeatherSnapshot } from './types/ipc';

export interface TaskWindow {
  hwnd: number;
  pid: number;
  title: string;
  exePath: string;
  exeName: string;
  monitorId: string;
  minimized: boolean;
  active: boolean;
}

export interface TaskGroup {
  key: string;
  displayName: string;
  exePath: string;
  pinned: boolean;
  active: boolean;
  windowCount: number;
  windows: TaskWindow[];
}

export interface TaskbarStatePayload {
  revision: number;
  sessionGeneration: number;
  groups: TaskGroup[];
  icons: Record<string, string>;
}

export interface ShellMonitorView {
  monitor: MonitorInfo;
  covered: boolean;
  windowLabel: string;
  barPlacement: { x: number; y: number; width: number; height: number };
}

export interface ShellSnapshot {
  revision: number;
  sessionGeneration: number;
  topologyRevision: number;
  helperConnected: boolean;
  syntheticBarsEnabled: boolean;
  barEdge: 'left' | 'right' | 'top' | 'bottom';
  barThicknessLogical: number;
  monitors: ShellMonitorView[];
  taskCount: number;
  lastRecovery: string;
}

export interface BarBridge {
  getSnapshot: () => Promise<ShellSnapshot>;
  getDashboardSnapshot?: () => Promise<DashboardSnapshot>;
  getTaskbarState: () => Promise<TaskbarStatePayload>;
  getWeather?: () => Promise<WeatherSnapshot | null>;
  onSnapshot: (handler: (snapshot: ShellSnapshot) => void) => Promise<UnlistenFn>;
  onTasks: (handler: (payload: TaskbarStatePayload) => void) => Promise<UnlistenFn>;
  activateWindow: (hwnd: number, pid: number, allowMinimize: boolean) => Promise<void>;
  closeWindow: (hwnd: number, pid: number) => Promise<void>;
  togglePin: (exePath: string, exeName: string, displayName?: string) => Promise<boolean>;
  launchApp: (path: string) => Promise<void>;
  toggleLauncher: () => Promise<void>;
  openDashboard: (monitorId: string | null) => Promise<void>;
  showPreview?: (
    monitorId: string | null,
    groupKey: string,
    anchorY: number,
    mode?: 'preview' | 'context',
  ) => Promise<void>;
  sessionAction?: (action: 'lock' | 'restoreShell' | 'quit') => Promise<void>;
}

const defaultBridge: BarBridge = {
  getSnapshot: () => invoke<ShellSnapshot>('shell_get_snapshot'),
  getDashboardSnapshot: () => invoke<DashboardSnapshot>('dashboard_get_snapshot'),
  getTaskbarState: () => invoke<TaskbarStatePayload>('taskbar_get_state'),
  getWeather: () => invoke<WeatherSnapshot | null>('dashboard_refresh_weather', { force: false }),
  onSnapshot: handler => listen<ShellSnapshot>('shell://snapshot', event => handler(event.payload)),
  onTasks: handler => listen<TaskbarStatePayload>('shell://tasks', event => handler(event.payload)),
  activateWindow: (hwnd, pid, allowMinimize) =>
    invoke<void>('taskbar_activate_window', { hwnd, pid, allowMinimize }),
  closeWindow: (hwnd, pid) => invoke<void>('taskbar_close_window', { hwnd, pid }),
  togglePin: (exePath, exeName, displayName) =>
    invoke<boolean>('taskbar_toggle_pin', { exePath, exeName, displayName }),
  launchApp: path => invoke<void>('taskbar_launch_app', { path }),
  toggleLauncher: () => invoke<void>('launcher_toggle'),
  openDashboard: monitorId =>
    invoke<void>('dashboard_set_visibility', { open: true, monitorId, trigger: 'click' }),
  showPreview: (monitorId, groupKey, anchorY, mode = 'preview') =>
    invoke<void>('preview_show', { monitorId, groupKey, anchorY, mode }),
  sessionAction: action => invoke<void>('shell_session_action', { action }),
};

export interface BarAppProps {
  monitorLabel: string;
  bridge?: BarBridge;
}

export function BarApp({ monitorLabel, bridge = defaultBridge }: BarAppProps) {
  const [snapshot, setSnapshot] = useState<ShellSnapshot | null>(null);
  const [taskbar, setTaskbar] = useState<TaskbarStatePayload | null>(null);
  const [weather, setWeather] = useState<WeatherSnapshot | null>(null);
  const [now, setNow] = useState(() => new Date());
  const hoverTimerRef = useRef<number | null>(null);

  useEffect(() => {
    let active = true;
    const unlisteners: UnlistenFn[] = [];

    const applySnap = (next: ShellSnapshot) => {
      if (!active) return;
      setSnapshot(prev => (prev && next.revision < prev.revision ? prev : next));
    };

    bridge.getSnapshot().then(applySnap).catch(() => {});
    bridge
      .getDashboardSnapshot?.()
      .then(dash => {
        if (active) applyMaterialTheme(dash.settings);
      })
      .catch(() => {});
    bridge
      .getTaskbarState()
      .then(data => {
        if (active) setTaskbar(data);
      })
      .catch(() => {});
    bridge
      .getWeather?.()
      .then(w => {
        if (active) setWeather(w);
      })
      .catch(() => {});

    Promise.all([
      bridge.onSnapshot(applySnap),
      bridge.onTasks(payload => {
        if (active) setTaskbar(payload);
      }),
    ])
      .then(fns => {
        if (!active) fns.forEach(fn => fn());
        else unlisteners.push(...fns);
      })
      .catch(() => {});

    const clockTimer = window.setInterval(() => setNow(new Date()), 15_000);

    return () => {
      active = false;
      window.clearInterval(clockTimer);
      unlisteners.forEach(fn => fn());
    };
  }, [bridge]);

  const view =
    snapshot?.monitors.find(m => m.windowLabel === monitorLabel) ?? snapshot?.monitors[0] ?? null;

  const handleGroupClick = (group: TaskGroup) => {
    if (group.windowCount === 0 && group.exePath) {
      void bridge.launchApp(group.exePath);
      return;
    }
    const target = group.windows.find(w => w.active) ?? group.windows[0];
    if (target) {
      void bridge.activateWindow(target.hwnd, target.pid, group.windowCount === 1);
    }
  };

  const groups = taskbar?.groups ?? [];

  return (
    <nav className="bar" data-testid="stellaria-bar" onContextMenu={e => e.preventDefault()}>
      <div className="section start">
        <button
          type="button"
          className="logo state-layer"
          title="App Launcher"
          onClick={() => void bridge.toggleLauncher()}
        >
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <path d="M3 5.1 10.1 4.1v6.8H3zM11 4l10-1.4v8.3H11zM3 11.9h7.1v6.9L3 17.8zM11 11.9h10v8.4L11 18.9z" />
          </svg>
        </button>

        <div className="group workspaces" aria-label="Workspace">
          <div className="workspace-indicator" style={{ top: 8, height: 22 }} />
          <div className="workspace focused">
            <span className="workspace-dot" />
          </div>
        </div>

        <div className="group tasks" data-testid="bar-task-list" hidden={groups.length === 0}>
          {groups.map(group => {
            const iconUrl = taskbar?.icons[group.key];
            const allMinimized =
              group.windowCount > 0 && group.windows.every(w => w.minimized);
            return (
              <button
                key={group.key}
                type="button"
                className={`task state-layer${group.active ? ' active' : ''}${allMinimized ? ' minimized' : ''}`}
                title={`${group.displayName}${group.windowCount > 0 ? ` (${group.windowCount})` : ''}`}
                onClick={() => handleGroupClick(group)}
                onMouseEnter={e => {
                  if (hoverTimerRef.current) window.clearTimeout(hoverTimerRef.current);
                  if (group.windowCount > 0 && bridge.showPreview) {
                    const rect = e.currentTarget.getBoundingClientRect();
                    const anchorY = rect.top + rect.height / 2;
                    hoverTimerRef.current = window.setTimeout(() => {
                      void bridge.showPreview?.(view?.monitor.id ?? null, group.key, anchorY, 'preview');
                    }, 220);
                  }
                }}
                onMouseLeave={() => {
                  if (hoverTimerRef.current) {
                    window.clearTimeout(hoverTimerRef.current);
                    hoverTimerRef.current = null;
                  }
                }}
                onContextMenu={e => {
                  e.preventDefault();
                  if (bridge.showPreview) {
                    const rect = e.currentTarget.getBoundingClientRect();
                    void bridge.showPreview(
                      view?.monitor.id ?? null,
                      group.key,
                      rect.top + rect.height / 2,
                      'context',
                    );
                  }
                }}
              >
                {iconUrl ? <img src={iconUrl} alt="" /> : <Icon name="web_asset" />}
                {group.windowCount > 1 && (
                  <span
                    style={{
                      position: 'absolute',
                      top: -2,
                      right: -2,
                      minWidth: 14,
                      height: 14,
                      padding: '0 3px',
                      borderRadius: 999,
                      background: 'var(--m3-primary)',
                      color: 'var(--m3-on-primary)',
                      fontSize: 9.5,
                      fontWeight: 700,
                      display: 'grid',
                      placeItems: 'center',
                    }}
                  >
                    {group.windowCount}
                  </span>
                )}
              </button>
            );
          })}
        </div>
      </div>

      <div className="section end">
        <button
          type="button"
          className="group weather state-layer"
          title="Weather"
          hidden={!weather}
          onClick={() => void bridge.openDashboard(view?.monitor.id ?? null)}
        >
          <Icon name={weather?.current.icon ?? 'cloud'} />
          <span className="weather-temp">
            {weather ? `${Math.round(weather.current.temperature)}°` : '--°'}
          </span>
        </button>

        <button
          type="button"
          className="group clock state-layer"
          title="Dashboard"
          onClick={() => void bridge.openDashboard(view?.monitor.id ?? null)}
        >
          <Icon name="calendar_month" />
          <span className="clock-digits">{String(now.getHours()).padStart(2, '0')}</span>
          <span className="clock-digits">{String(now.getMinutes()).padStart(2, '0')}</span>
        </button>

        <div className="group status">
          <button
            type="button"
            className="status-item keyboard state-layer"
            title="Language"
            onClick={() => void bridge.openDashboard(view?.monitor.id ?? null)}
          >
            US
          </button>
          <button
            type="button"
            className="status-item state-layer"
            title="Audio"
            onClick={() => void bridge.openDashboard(view?.monitor.id ?? null)}
          >
            <Icon name="volume_up" />
          </button>
          <button
            type="button"
            className="status-item state-layer"
            title="Network"
            onClick={() => void bridge.openDashboard(view?.monitor.id ?? null)}
          >
            <Icon name="lan" />
          </button>
          <button
            type="button"
            className="status-item notifications state-layer"
            title="Notifications"
            onClick={() => void bridge.openDashboard(view?.monitor.id ?? null)}
          >
            <Icon name="notifications" />
          </button>
        </div>

        <button
          type="button"
          className="power state-layer"
          title="Lock workstation"
          onClick={() => void bridge.sessionAction?.('lock')}
        >
          <Icon name="power_settings_new" />
        </button>
      </div>
    </nav>
  );
}
