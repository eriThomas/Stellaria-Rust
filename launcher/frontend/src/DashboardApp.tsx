import { useEffect } from 'react';
import type { DashboardBridge } from './api/tauriBridge';
import { tauriDashboardBridge } from './api/tauriBridge';
import { useDashboardStore } from './store/dashboardStore';
import type { DashboardTab } from './types/ipc';

const TABS: Array<{ id: DashboardTab; label: string }> = [
  { id: 'dash', label: 'Dashboard' },
  { id: 'media', label: 'Media' },
  { id: 'performance', label: 'Performance' },
  { id: 'weather', label: 'Weather' },
];

export interface DashboardAppProps {
  bridge?: DashboardBridge;
}

export function DashboardApp({ bridge = tauriDashboardBridge }: DashboardAppProps) {
  const visibility = useDashboardStore(state => state.visibility);
  const settings = useDashboardStore(state => state.settings);
  const hydrated = useDashboardStore(state => state.hydrated);
  const applySnapshot = useDashboardStore(state => state.applySnapshot);
  const applyVisibilityEvent = useDashboardStore(state => state.applyVisibilityEvent);
  const applySettingsUpdate = useDashboardStore(state => state.applySettingsUpdate);

  useEffect(() => {
    let active = true;
    const unlisteners: Array<() => void> = [];

    bridge
      .getSnapshot()
      .then(snapshot => {
        if (active) applySnapshot(snapshot);
      })
      .catch(() => {});

    Promise.all([
      bridge.onVisibility(event => {
        if (active) applyVisibilityEvent(event);
      }),
      bridge.onSettings(event => {
        if (active) applySettingsUpdate(event.revision, event.settings);
      }),
    ])
      .then(fns => {
        if (!active) {
          fns.forEach(fn => fn());
        } else {
          unlisteners.push(...fns);
        }
      })
      .catch(() => {});

    return () => {
      active = false;
      unlisteners.forEach(fn => fn());
    };
  }, [bridge, applySnapshot, applyVisibilityEvent, applySettingsUpdate]);

  useEffect(() => {
    if (!visibility.open) return;
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        event.preventDefault();
        void bridge.setVisibility(false, visibility.monitorId, 'hotkey');
      }
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [bridge, visibility.open, visibility.monitorId]);

  const handleTabSelect = (tab: DashboardTab) => {
    void bridge.setActiveTab(tab).then(applyVisibilityEvent);
  };

  return (
    <main
      className={`dashboard-shell${visibility.open ? ' is-open' : ' is-hidden'}${settings.reducedMotion || settings.gameMode ? ' reduced-motion' : ''}`}
      data-theme-mode={settings.themeMode}
      data-active-tab={visibility.activeTab}
      aria-hidden={!visibility.open}
    >
      <nav className="dashboard-tabs" aria-label="Dashboard sections">
        {TABS.map(tab => (
          <button
            key={tab.id}
            type="button"
            role="tab"
            aria-selected={visibility.activeTab === tab.id}
            className={visibility.activeTab === tab.id ? 'tab current' : 'tab'}
            onClick={() => handleTabSelect(tab.id)}
          >
            {tab.label}
          </button>
        ))}
      </nav>
      <section className="dashboard-viewport" role="tabpanel">
        {!hydrated ? (
          <p>Connecting to Stellaria core…</p>
        ) : !visibility.open ? null : (
          <div data-testid={`tab-panel-${visibility.activeTab}`}>
            <h2>{TABS.find(t => t.id === visibility.activeTab)?.label}</h2>
            <p>Milestone 0 shell host ready on monitor {visibility.monitorId ?? 'primary'}.</p>
          </div>
        )}
      </section>
    </main>
  );
}
