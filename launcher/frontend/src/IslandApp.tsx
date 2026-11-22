import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useEffect, useState } from 'react';
import { Icon } from './components/ExpressiveShape';
import { applyMaterialTheme } from './theme/materialYou';
import type { DashboardSnapshot } from './types/ipc';

export interface IslandToastPayload {
  label: string;
  title: string;
  subtitle?: string;
  icon?: string;
}

export function IslandApp() {
  const [toast, setToast] = useState<IslandToastPayload>({
    label: 'Stellaria',
    title: '',
    subtitle: '',
    icon: 'notifications',
  });

  useEffect(() => {
    invoke<DashboardSnapshot>('dashboard_get_snapshot')
      .then(s => applyMaterialTheme(s.settings))
      .catch(() => {});

    document.body.classList.add('open');

    const unlistenShow = listen<IslandToastPayload>('island://show', ev => {
      setToast(ev.payload);
      document.body.classList.remove('closing');
      document.body.classList.add('open');
    });

    const unlistenHide = listen<void>('island://hide', () => {
      document.body.classList.remove('open');
      document.body.classList.add('closing');
    });

    return () => {
      void unlistenShow.then(fn => fn());
      void unlistenHide.then(fn => fn());
    };
  }, []);

  return (
    <div className="drawer from-top" style={{ width: '100%', height: '100%' }}>
      <div className="drawer-anchor">
        <div className="flare start" />
        <section className="drawer-panel island" id="panel">
          <div className="drawer-content island-content">
            <div className="island-notification">
              <div className="island-art">
                <Icon name={toast.icon || 'notifications'} />
              </div>
              <div className="island-text">
                <span className="island-label">{toast.label}</span>
                <span className="island-title">{toast.title}</span>
                {toast.subtitle && <span className="island-subtitle">{toast.subtitle}</span>}
              </div>
            </div>
          </div>
        </section>
        <div className="flare end" />
      </div>
    </div>
  );
}
