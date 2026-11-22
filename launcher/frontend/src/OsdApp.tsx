import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useEffect, useState } from 'react';
import { Icon } from './components/ExpressiveShape';
import { applyMaterialTheme } from './theme/materialYou';
import type { DashboardSnapshot } from './types/ipc';

interface OsdPayload {
  kind: 'volume' | 'brightness';
  value: number;
  muted: boolean;
}

export function OsdApp() {
  const [payload, setPayload] = useState<OsdPayload>({ kind: 'volume', value: 50, muted: false });

  useEffect(() => {
    invoke<DashboardSnapshot>('dashboard_get_snapshot')
      .then(s => applyMaterialTheme(s.settings))
      .catch(() => {});

    document.body.classList.add('open');

    const unlistenShow = listen<OsdPayload>('osd://show', ev => {
      setPayload(ev.payload);
      document.body.classList.remove('closing');
      document.body.classList.add('open');
    });

    const unlistenHide = listen<void>('osd://hide', () => {
      document.body.classList.remove('open');
      document.body.classList.add('closing');
    });

    return () => {
      void unlistenShow.then(fn => fn());
      void unlistenHide.then(fn => fn());
    };
  }, []);

  const pct = payload.muted ? 0 : Math.min(100, Math.max(0, payload.value));
  const iconName =
    payload.kind === 'brightness'
      ? 'light_mode'
      : payload.muted || pct === 0
        ? 'volume_off'
        : pct < 34
          ? 'volume_mute'
          : pct < 67
            ? 'volume_down'
            : 'volume_up';

  return (
    <div className="drawer from-right" style={{ width: '100%', height: '100%' }}>
      <div className="drawer-anchor">
        <div className="flare start" />
        <section className="drawer-panel osd" id="panel">
          <div className="drawer-content osd-content">
            <div
              className={`vslider${payload.muted ? ' muted' : ''}`}
              onWheel={e => {
                e.preventDefault();
                if (payload.kind === 'volume') {
                  void invoke('shell_volume_action', { action: e.deltaY < 0 ? 'up' : 'down' });
                }
              }}
              onClick={() => {
                if (payload.kind === 'volume') {
                  void invoke('shell_volume_action', { action: 'mute' });
                }
              }}
            >
              <div className="vslider-fill" style={{ height: `${pct}%` }} />
              <Icon name={iconName} />
            </div>
          </div>
        </section>
        <div className="flare end" />
      </div>
    </div>
  );
}
