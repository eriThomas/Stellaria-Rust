import { useCallback, useEffect, useState } from 'react';
import type { DashboardBridge } from './api/tauriBridge';
import { tauriDashboardBridge } from './api/tauriBridge';
import { HomeTab } from './components/HomeTab';
import { MediaTab } from './components/MediaTab';
import { PerformanceTab } from './components/PerformanceTab';
import { SettingsPopover } from './components/SettingsPopover';
import { WeatherTab } from './components/WeatherTab';
import { resolveLanguage, translate } from './i18n/messages';
import { useDashboardStore } from './store/dashboardStore';
import { applyMaterialTheme } from './theme/materialYou';
import type { DashboardSettingsPatch, DashboardTab, MediaAction } from './types/ipc';

const TABS: Array<{ id: DashboardTab; labelKey: string }> = [
  { id: 'dash', labelKey: 'tab_dash' },
  { id: 'media', labelKey: 'tab_media' },
  { id: 'performance', labelKey: 'tab_performance' },
  { id: 'weather', labelKey: 'tab_weather' },
];

export interface DashboardAppProps {
  bridge?: DashboardBridge;
}

export function DashboardApp({ bridge = tauriDashboardBridge }: DashboardAppProps) {
  const [settingsOpen, setSettingsOpen] = useState(false);

  const visibility = useDashboardStore(state => state.visibility);
  const settings = useDashboardStore(state => state.settings);
  const hydrated = useDashboardStore(state => state.hydrated);
  const sysinfo = useDashboardStore(state => state.sysinfo);
  const telemetry = useDashboardStore(state => state.telemetry);
  const networkHistory = useDashboardStore(state => state.networkHistory);
  const media = useDashboardStore(state => state.media);
  const weather = useDashboardStore(state => state.weather);
  const weatherError = useDashboardStore(state => state.weatherError);
  const lyrics = useDashboardStore(state => state.lyrics);

  const applySnapshot = useDashboardStore(state => state.applySnapshot);
  const applyVisibilityEvent = useDashboardStore(state => state.applyVisibilityEvent);
  const applySettingsUpdate = useDashboardStore(state => state.applySettingsUpdate);
  const applyTelemetry = useDashboardStore(state => state.applyTelemetry);
  const setSysInfo = useDashboardStore(state => state.setSysInfo);
  const setMedia = useDashboardStore(state => state.setMedia);
  const setWeather = useDashboardStore(state => state.setWeather);
  const setLyrics = useDashboardStore(state => state.setLyrics);

  const lang = resolveLanguage(settings.language);

  useEffect(() => {
    applyMaterialTheme(settings);
  }, [settings]);

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
      bridge.onTelemetry(snapshot => {
        if (active) applyTelemetry(snapshot);
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
  }, [bridge, applySnapshot, applyVisibilityEvent, applySettingsUpdate, applyTelemetry]);

  const refreshWeather = useCallback(
    (force = false) => {
      if (!settings.weatherEnabled) {
        setWeather(null, null);
        return;
      }
      bridge
        .refreshWeather(force)
        .then(data => setWeather(data, null))
        .catch(() => setWeather(null, 'weather-unavailable'));
    },
    [bridge, settings.weatherEnabled, setWeather],
  );

  // Lazy-load sysinfo, telemetry, media, and weather ONLY while visible.
  useEffect(() => {
    if (!visibility.open) {
      setSettingsOpen(false);
      return;
    }
    let cancelled = false;

    if (!sysinfo) {
      bridge
        .getSysInfo()
        .then(info => {
          if (!cancelled) setSysInfo(info);
        })
        .catch(() => {});
    }

    bridge
      .getTelemetry()
      .then(snap => {
        if (!cancelled) applyTelemetry(snap);
      })
      .catch(() => {});

    bridge
      .getMedia()
      .then(snap => {
        if (!cancelled) setMedia(snap);
      })
      .catch(() => {});

    refreshWeather(false);

    return () => {
      cancelled = true;
    };
  }, [visibility.open, sysinfo, bridge, setSysInfo, applyTelemetry, setMedia, refreshWeather]);

  // Fetch synced lyrics only when visible, on Media tab, and lyrics enabled.
  const currentTitle = media?.current?.title ?? '';
  const currentArtist = media?.current?.artist ?? '';
  const currentDuration = media?.current?.durationSecs ?? 0;
  useEffect(() => {
    if (!visibility.open || visibility.activeTab !== 'media' || !settings.lyricsEnabled || !currentTitle) {
      return;
    }
    let cancelled = false;
    setLyrics(null);
    bridge
      .getLyrics(currentArtist, currentTitle, currentDuration > 0 ? currentDuration : undefined)
      .then(result => {
        if (!cancelled) setLyrics(result);
      })
      .catch(() => {
        if (!cancelled) setLyrics({ key: `${currentArtist}|${currentTitle}`, lines: [] });
      });
    return () => {
      cancelled = true;
    };
  }, [
    visibility.open,
    visibility.activeTab,
    settings.lyricsEnabled,
    currentArtist,
    currentTitle,
    currentDuration,
    bridge,
    setLyrics,
  ]);

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

  const handleMediaAction = (action: MediaAction) => {
    void bridge.sendMediaAction(action).then(() => bridge.getMedia().then(setMedia).catch(() => {}));
  };

  const handleSettingsUpdate = (patch: DashboardSettingsPatch) => {
    void bridge.updateSettings(patch).then(updated => {
      applySettingsUpdate(useDashboardStore.getState().revision + 1, updated);
    });
  };

  return (
    <main
      className={`dashboard-shell${visibility.open ? ' is-open' : ' is-hidden'}${settings.reducedMotion || settings.gameMode ? ' reduced-motion' : ''}`}
      data-theme-mode={settings.themeMode}
      data-active-tab={visibility.activeTab}
      aria-hidden={!visibility.open}
    >
      <header className="dashboard-header">
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
              {translate(lang, tab.labelKey)}
            </button>
          ))}
        </nav>

        <button
          type="button"
          className="icon-btn"
          aria-expanded={settingsOpen}
          aria-label={translate(lang, 'settings')}
          onClick={() => setSettingsOpen(open => !open)}
        >
          ⚙ {translate(lang, 'settings')}
        </button>
      </header>

      {settingsOpen && visibility.open && (
        <SettingsPopover lang={lang} settings={settings} onUpdate={handleSettingsUpdate} />
      )}

      <section className="dashboard-viewport" role="tabpanel">
        {!hydrated ? (
          <p>{translate(lang, 'loading')}</p>
        ) : !visibility.open ? null : visibility.activeTab === 'dash' ? (
          <HomeTab
            lang={lang}
            isOpen={visibility.open}
            clockSeconds={settings.clockSeconds}
            weatherEnabled={settings.weatherEnabled}
            weather={weather}
            weatherError={weatherError}
            sysinfo={sysinfo}
            telemetry={telemetry}
            media={media}
            onMediaAction={handleMediaAction}
          />
        ) : visibility.activeTab === 'media' ? (
          <MediaTab
            lang={lang}
            media={media}
            lyricsEnabled={settings.lyricsEnabled}
            lyrics={lyrics}
            onMediaAction={handleMediaAction}
          />
        ) : visibility.activeTab === 'performance' ? (
          <PerformanceTab
            lang={lang}
            sysinfo={sysinfo}
            telemetry={telemetry}
            networkHistory={networkHistory}
          />
        ) : (
          <WeatherTab
            lang={lang}
            weatherEnabled={settings.weatherEnabled}
            weather={weather}
            weatherError={weatherError}
            onRefresh={() => refreshWeather(true)}
          />
        )}
      </section>
    </main>
  );
}
