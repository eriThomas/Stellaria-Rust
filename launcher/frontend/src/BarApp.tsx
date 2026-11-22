import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { useEffect, useState } from 'react';
import type { MonitorInfo } from './types/ipc';

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
  onSnapshot: (handler: (snapshot: ShellSnapshot) => void) => Promise<UnlistenFn>;
}

const defaultBridge: BarBridge = {
  getSnapshot: () => invoke<ShellSnapshot>('shell_get_snapshot'),
  onSnapshot: handler => listen<ShellSnapshot>('shell://snapshot', event => handler(event.payload)),
};

export interface BarAppProps {
  monitorLabel: string;
  bridge?: BarBridge;
}

export function BarApp({ monitorLabel, bridge = defaultBridge }: BarAppProps) {
  const [snapshot, setSnapshot] = useState<ShellSnapshot | null>(null);

  useEffect(() => {
    let active = true;
    let unlisten: UnlistenFn | null = null;

    const apply = (next: ShellSnapshot) => {
      if (!active) return;
      setSnapshot(prev => (prev && next.revision < prev.revision ? prev : next));
    };

    bridge.getSnapshot().then(apply).catch(() => {});
    bridge
      .onSnapshot(apply)
      .then(fn => {
        if (!active) fn();
        else unlisten = fn;
      })
      .catch(() => {});

    return () => {
      active = false;
      unlisten?.();
    };
  }, [bridge]);

  const view =
    snapshot?.monitors.find(m => m.windowLabel === monitorLabel) ?? snapshot?.monitors[0] ?? null;

  return (
    <aside
      data-testid="synthetic-bar"
      style={{
        width: '100vw',
        height: '100vh',
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        justifyContent: 'space-between',
        padding: '10px 4px',
        background: 'rgba(20, 18, 24, 0.78)',
        borderRight: '2px solid rgba(186, 155, 255, 0.65)',
        color: '#e6e0e9',
        fontFamily: 'Segoe UI, system-ui, sans-serif',
        fontSize: 10,
        textAlign: 'center',
        boxSizing: 'border-box',
        pointerEvents: 'none',
      }}
    >
      <div>
        <div style={{ fontWeight: 700, color: '#d0bcff' }}>M2</div>
        <div data-testid="bar-monitor-id" style={{ wordBreak: 'break-all' }}>
          {view?.monitor.name.replace(/^\\\\\.\\/, '') ?? monitorLabel}
        </div>
        {view?.monitor.primary && <div style={{ color: '#86efac' }}>PRI</div>}
      </div>

      <div>
        <div>{view ? `${Math.round(view.monitor.scaleFactor * 100)}%` : '—'}</div>
        <div data-testid="bar-task-count">{snapshot ? `${snapshot.taskCount}t` : '0t'}</div>
        <div style={{ color: snapshot?.helperConnected ? '#86efac' : '#fca5a5' }}>
          {snapshot?.helperConnected ? 'WS' : 'OFF'}
        </div>
      </div>

      <div style={{ opacity: 0.8 }}>
        <div data-testid="bar-gen">g{snapshot?.sessionGeneration ?? 0}</div>
        <div data-testid="bar-rev">r{snapshot?.topologyRevision ?? 0}</div>
      </div>
    </aside>
  );
}
