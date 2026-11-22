import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useEffect, useState } from 'react';
import { Icon } from './components/ExpressiveShape';
import { applyMaterialTheme } from './theme/materialYou';
import type { DashboardSnapshot } from './types/ipc';

type SessionAction = 'lock' | 'sleep' | 'logout' | 'restart' | 'shutdown';

const BUTTONS: Array<{ action: SessionAction; icon: string; title: string; danger?: boolean }> = [
  { action: 'lock', icon: 'lock', title: 'Lock' },
  { action: 'sleep', icon: 'bedtime', title: 'Sleep' },
  { action: 'logout', icon: 'logout', title: 'Sign out' },
  { action: 'restart', icon: 'restart_alt', title: 'Restart', danger: true },
  { action: 'shutdown', icon: 'power_settings_new', title: 'Shut down', danger: true },
];

export function SessionApp() {
  const [focusedIndex, setFocusedIndex] = useState<number>(-1);

  useEffect(() => {
    invoke<DashboardSnapshot>('dashboard_get_snapshot')
      .then(s => applyMaterialTheme(s.settings))
      .catch(() => {});

    document.body.classList.add('open');
    const unlistenPromise = listen<boolean>('session://visibility', ev => {
      if (ev.payload) {
        document.body.classList.remove('closing');
        document.body.classList.add('open');
        setFocusedIndex(-1);
      } else {
        document.body.classList.remove('open');
        document.body.classList.add('closing');
      }
    });

    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        void invoke('session_hide');
      } else if (e.key === 'ArrowDown') {
        e.preventDefault();
        setFocusedIndex(i => (i + 1) % BUTTONS.length);
      } else if (e.key === 'ArrowUp') {
        e.preventDefault();
        setFocusedIndex(i => (i <= 0 ? BUTTONS.length - 1 : i - 1));
      } else if (e.key === 'Enter' && focusedIndex >= 0 && focusedIndex < BUTTONS.length) {
        e.preventDefault();
        void invoke('session_hide');
        void invoke('shell_session_action', { action: BUTTONS[focusedIndex].action });
      }
    };
    const onBlur = () => {
      void invoke('session_hide');
    };
    window.addEventListener('keydown', onKeyDown);
    window.addEventListener('blur', onBlur);
    return () => {
      window.removeEventListener('keydown', onKeyDown);
      window.removeEventListener('blur', onBlur);
      void unlistenPromise.then(fn => fn());
    };
  }, [focusedIndex]);

  const trigger = (action: SessionAction) => {
    void invoke('session_hide');
    void invoke('shell_session_action', { action });
  };

  return (
    <div
      className="drawer from-right"
      style={{ width: '100%', height: '100%' }}
      onMouseDown={e => {
        if (e.target === e.currentTarget) void invoke('session_hide');
      }}
    >
      <div className="drawer-anchor">
        <div className="flare start" />
        <section className="drawer-panel session" id="panel">
          <div className="drawer-content session-content cascade">
            {BUTTONS.map((btn, idx) => (
              <button
                key={btn.action}
                type="button"
                className={`session-button state-layer${btn.danger ? ' danger' : ''}${focusedIndex === idx ? ' focused' : ''}`}
                title={btn.title}
                aria-label={btn.title}
                onMouseEnter={() => setFocusedIndex(idx)}
                onMouseLeave={() => setFocusedIndex(-1)}
                onClick={() => trigger(btn.action)}
              >
                <Icon name={btn.icon} />
              </button>
            ))}
          </div>
        </section>
        <div className="flare end" />
      </div>
    </div>
  );
}
