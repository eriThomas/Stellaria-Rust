import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { useEffect, useState } from 'react';
import { Icon } from './components/ExpressiveShape';
import type { MonitorInfo } from './types/ipc';

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
  getTaskbarState: () => Promise<TaskbarStatePayload>;
  onSnapshot: (handler: (snapshot: ShellSnapshot) => void) => Promise<UnlistenFn>;
  onTasks: (handler: (payload: TaskbarStatePayload) => void) => Promise<UnlistenFn>;
  activateWindow: (hwnd: number, pid: number, allowMinimize: boolean) => Promise<void>;
  closeWindow: (hwnd: number, pid: number) => Promise<void>;
  togglePin: (exePath: string, exeName: string, displayName?: string) => Promise<boolean>;
  launchApp: (path: string) => Promise<void>;
  toggleLauncher: () => Promise<void>;
  openDashboard: (monitorId: string | null) => Promise<void>;
}

const defaultBridge: BarBridge = {
  getSnapshot: () => invoke<ShellSnapshot>('shell_get_snapshot'),
  getTaskbarState: () => invoke<TaskbarStatePayload>('taskbar_get_state'),
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
};

export interface BarAppProps {
  monitorLabel: string;
  bridge?: BarBridge;
}

export function BarApp({ monitorLabel, bridge = defaultBridge }: BarAppProps) {
  const [snapshot, setSnapshot] = useState<ShellSnapshot | null>(null);
  const [taskbar, setTaskbar] = useState<TaskbarStatePayload | null>(null);
  const [contextGroup, setContextGroup] = useState<TaskGroup | null>(null);
  const [now, setNow] = useState(() => new Date());

  useEffect(() => {
    let active = true;
    const unlisteners: UnlistenFn[] = [];

    const applySnap = (next: ShellSnapshot) => {
      if (!active) return;
      setSnapshot(prev => (prev && next.revision < prev.revision ? prev : next));
    };

    bridge.getSnapshot().then(applySnap).catch(() => {});
    bridge
      .getTaskbarState()
      .then(data => {
        if (active) setTaskbar(data);
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
    setContextGroup(null);
    if (group.windowCount === 0 && group.exePath) {
      void bridge.launchApp(group.exePath);
      return;
    }
    const target = group.windows.find(w => w.active) ?? group.windows[0];
    if (target) {
      void bridge.activateWindow(target.hwnd, target.pid, group.windowCount === 1);
    }
  };

  return (
    <aside
      data-testid="stellaria-bar"
      onContextMenu={e => e.preventDefault()}
      style={{
        width: '100vw',
        height: '100vh',
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        justifyContent: 'space-between',
        padding: '10px 4px',
        background: 'rgba(18, 17, 22, 0.92)',
        borderRight: '1px solid rgba(255, 255, 255, 0.08)',
        color: '#e6e0e9',
        fontFamily: 'var(--font-sans)',
        boxSizing: 'border-box',
        position: 'relative',
      }}
    >
      {/* Top: Start Launcher + Dashboard */}
      <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 8 }}>
        <button
          type="button"
          title="App Launcher"
          onClick={() => void bridge.toggleLauncher()}
          style={{
            width: 36,
            height: 36,
            borderRadius: 12,
            display: 'grid',
            placeItems: 'center',
            background: 'rgba(208, 188, 255, 0.14)',
            color: '#d0bcff',
          }}
        >
          <Icon name="apps" filled />
        </button>
        <button
          type="button"
          title="Open Dashboard"
          onClick={() => void bridge.openDashboard(view?.monitor.id ?? null)}
          style={{
            width: 36,
            height: 36,
            borderRadius: 12,
            display: 'grid',
            placeItems: 'center',
            background: 'rgba(255, 255, 255, 0.06)',
            color: '#cac4d0',
          }}
        >
          <Icon name="dashboard" />
        </button>
      </div>

      {/* Middle: Grouped + Pinned Tasks */}
      <div
        data-testid="bar-task-list"
        style={{
          flex: 1,
          minHeight: 0,
          overflowY: 'auto',
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          gap: 6,
          padding: '8px 0',
          scrollbarWidth: 'none',
        }}
      >
        {(taskbar?.groups ?? []).map(group => {
          const iconUrl = taskbar?.icons[group.key];
          return (
            <button
              key={group.key}
              type="button"
              title={`${group.displayName}${group.windowCount > 0 ? ` (${group.windowCount})` : ''}`}
              onClick={() => handleGroupClick(group)}
              onContextMenu={e => {
                e.preventDefault();
                setContextGroup(curr => (curr?.key === group.key ? null : group));
              }}
              style={{
                width: 38,
                height: 38,
                borderRadius: 14,
                position: 'relative',
                display: 'grid',
                placeItems: 'center',
                background: group.active
                  ? 'rgba(208, 188, 255, 0.24)'
                  : group.windowCount > 0
                    ? 'rgba(255, 255, 255, 0.07)'
                    : 'transparent',
                border: group.active ? '1px solid rgba(208, 188, 255, 0.55)' : '1px solid transparent',
              }}
            >
              {iconUrl ? (
                <img src={iconUrl} alt="" style={{ width: 22, height: 22, objectFit: 'contain' }} />
              ) : (
                <Icon name="web_asset" />
              )}
              {group.windowCount > 1 && (
                <span
                  style={{
                    position: 'absolute',
                    top: -2,
                    right: -2,
                    minWidth: 15,
                    height: 15,
                    padding: '0 3px',
                    borderRadius: 999,
                    background: '#d0bcff',
                    color: '#1e192b',
                    fontSize: 10,
                    fontWeight: 700,
                    display: 'grid',
                    placeItems: 'center',
                  }}
                >
                  {group.windowCount}
                </span>
              )}
              {group.windowCount > 0 && (
                <span
                  style={{
                    position: 'absolute',
                    left: 1,
                    width: 3,
                    height: group.active ? 16 : 6,
                    borderRadius: 999,
                    background: group.active ? '#d0bcff' : '#938f99',
                  }}
                />
              )}
            </button>
          );
        })}
      </div>

      {/* Inline Context Actions Popover when right-clicking a task */}
      {contextGroup && (
        <div
          data-testid="bar-context-menu"
          style={{
            position: 'absolute',
            bottom: 64,
            left: 2,
            right: 2,
            background: '#2b2930',
            border: '1px solid rgba(255,255,255,0.16)',
            borderRadius: 12,
            padding: 4,
            display: 'flex',
            flexDirection: 'column',
            gap: 4,
            zIndex: 20,
          }}
        >
          <button
            type="button"
            title={contextGroup.pinned ? 'Unpin' : 'Pin'}
            onClick={() => {
              const exeName = contextGroup.windows[0]?.exeName || contextGroup.key;
              void bridge.togglePin(contextGroup.exePath, exeName, contextGroup.displayName);
              setContextGroup(null);
            }}
            style={{ padding: '4px 0', borderRadius: 8, background: 'rgba(255,255,255,0.08)' }}
          >
            <Icon name={contextGroup.pinned ? 'keep_off' : 'keep'} />
          </button>
          {contextGroup.windowCount > 0 && (
            <button
              type="button"
              title="Close all"
              onClick={() => {
                contextGroup.windows.forEach(w => void bridge.closeWindow(w.hwnd, w.pid));
                setContextGroup(null);
              }}
              style={{ padding: '4px 0', borderRadius: 8, background: 'rgba(242,184,181,0.18)', color: '#f2b8b5' }}
            >
              <Icon name="close" />
            </button>
          )}
        </div>
      )}

      {/* Bottom: Vertical Clock */}
      <button
        type="button"
        title="Open Dashboard"
        onClick={() => void bridge.openDashboard(view?.monitor.id ?? null)}
        style={{
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          lineHeight: 1.05,
          fontWeight: 700,
          fontSize: 13,
          color: '#e6e0e9',
          padding: '6px 0',
        }}
      >
        <span>{String(now.getHours()).padStart(2, '0')}</span>
        <span style={{ color: '#d0bcff' }}>{String(now.getMinutes()).padStart(2, '0')}</span>
      </button>
    </aside>
  );
}
