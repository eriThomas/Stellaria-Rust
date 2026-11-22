import { useCallback, useEffect, useLayoutEffect, useRef, useState } from 'react';
import type { DashboardBridge } from './api/tauriBridge';
import { tauriDashboardBridge } from './api/tauriBridge';
import { Icon } from './components/ExpressiveShape';
import { HomeTab } from './components/HomeTab';
import { MediaTab } from './components/MediaTab';
import { PerformanceTab } from './components/PerformanceTab';
import { SettingsPopover } from './components/SettingsPopover';
import { WeatherTab } from './components/WeatherTab';
import { resolveLanguage, translate } from './i18n/messages';
import { useDashboardStore } from './store/dashboardStore';
import { applyMaterialTheme } from './theme/materialYou';
import type { DashboardSettingsPatch, DashboardTab, MediaAction } from './types/ipc';

const TABS: Array<{ id: DashboardTab; labelKey: string; icon: string }> = [
  { id: 'dash', labelKey: 'tab_dash', icon: 'dashboard' },
  { id: 'media', labelKey: 'tab_media', icon: 'queue_music' },
  { id: 'performance', labelKey: 'tab_performance', icon: 'speed' },
  { id: 'weather', labelKey: 'tab_weather', icon: 'cloud' },
];

export interface DashboardAppProps {
  bridge?: DashboardBridge;
}

export function DashboardApp({ bridge = tauriDashboardBridge }: DashboardAppProps) {
  const [settingsOpen, setSettingsOpen] = useState(false);

  const viewportRef = useRef<HTMLDivElement | null>(null);
  const trackRef = useRef<HTMLDivElement | null>(null);
  const indicatorRef = useRef<HTMLSpanElement | null>(null);
  const tabRefs = useRef<Record<string, HTMLButtonElement | null>>({});

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

  // Drive Stellaria's native drawer open/closing CSS animations on document.body
  useEffect(() => {
    if (visibility.open) {
      document.body.classList.remove('closing');
      document.body.classList.add('open');
    } else {
      document.body.classList.remove('open');
      document.body.classList.add('closing');
      const timer = window.setTimeout(() => {
        document.body.classList.remove('closing');
      }, 360);
      return () => window.clearTimeout(timer);
    }
  }, [visibility.open]);

  const layoutTabs = useCallback(() => {
    const viewport = viewportRef.current;
    const track = trackRef.current;
    const indicator = indicatorRef.current;
    const activeTabButton = tabRefs.current[visibility.activeTab];
    if (!viewport || !track || !indicator || !activeTabButton) return;

    const activePage = track.querySelector<HTMLElement>(`.page[data-page="${visibility.activeTab}"]`);
    const inner = activeTabButton.querySelector<HTMLElement>('.tab-inner');
    if (activePage && activePage.offsetWidth > 0) {
      viewport.style.width = `${activePage.offsetWidth}px`;
      viewport.style.height = `${activePage.offsetHeight}px`;
      track.style.transform = `translateX(${-activePage.offsetLeft}px)`;
    }
    if (inner && activeTabButton.offsetWidth > 0) {
      indicator.style.width = `${inner.offsetWidth}px`;
      indicator.style.transform = `translateX(${
        activeTabButton.offsetLeft + (activeTabButton.offsetWidth - inner.offsetWidth) / 2
      }px)`;
    }
  }, [visibility.activeTab]);

  useLayoutEffect(() => {
    if (!visibility.open) return;
    layoutTabs();
    const raf = window.requestAnimationFrame(layoutTabs);
    return () => window.cancelAnimationFrame(raf);
  }, [visibility.open, visibility.activeTab, layoutTabs, weather, media, telemetry]);

  useEffect(() => {
    if (!visibility.open || !trackRef.current || typeof ResizeObserver === 'undefined') return;
    const observer = new ResizeObserver(() => layoutTabs());
    observer.observe(trackRef.current);
    return () => observer.disconnect();
  }, [visibility.open, layoutTabs]);

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

    const pollMedia = () => {
      bridge
        .getMedia()
        .then(snap => {
          if (!cancelled) setMedia(snap);
        })
        .catch(() => {});
    };

    pollMedia();
    const mediaTimer = window.setInterval(pollMedia, 2000);

    refreshWeather(false);

    return () => {
      cancelled = true;
      window.clearInterval(mediaTimer);
    };
  }, [visibility.open, sysinfo, bridge, setSysInfo, applyTelemetry, setMedia, refreshWeather]);

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
    <div className="drawer from-top" aria-hidden={!visibility.open}>
      <div className="drawer-anchor">
        <div className="flare start" />
        <section className="drawer-panel dashboard" style={{ position: 'relative' }}>
          <button
            type="button"
            className="dashboard-settings-trigger"
            aria-expanded={settingsOpen}
            aria-label={translate(lang, 'settings')}
            title={translate(lang, 'settings')}
            onClick={() => setSettingsOpen(open => !open)}
          >
            <Icon name="settings" />
          </button>

          {settingsOpen && visibility.open && (
            <SettingsPopover lang={lang} settings={settings} onUpdate={handleSettingsUpdate} />
          )}

          <div className="drawer-content dashboard-content">
            <nav
              className="tabs"
              aria-label="Dashboard sections"
              onWheel={event => {
                event.preventDefault();
                const idx = TABS.findIndex(t => t.id === visibility.activeTab);
                const next = Math.min(TABS.length - 1, Math.max(0, idx + (event.deltaY > 0 ? 1 : -1)));
                handleTabSelect(TABS[next].id);
              }}
            >
              <div className="tab-row">
                {TABS.map(tab => (
                  <button
                    key={tab.id}
                    ref={el => {
                      tabRefs.current[tab.id] = el;
                    }}
                    type="button"
                    role="tab"
                    aria-selected={visibility.activeTab === tab.id}
                    className={`tab${visibility.activeTab === tab.id ? ' current' : ''}`}
                    onClick={() => handleTabSelect(tab.id)}
                  >
                    <span className="tab-inner">
                      <Icon name={tab.icon} />
                      <span className="tab-label">{translate(lang, tab.labelKey)}</span>
                    </span>
                  </button>
                ))}
              </div>
              <span className="tab-indicator" ref={indicatorRef} />
              <span className="tab-separator" />
            </nav>

            <div className="viewport" ref={viewportRef}>
              <div className="track" ref={trackRef}>
                {hydrated && visibility.open && (
                  <>
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
                    <MediaTab
                      lang={lang}
                      media={media}
                      lyricsEnabled={settings.lyricsEnabled}
                      lyrics={lyrics}
                      onMediaAction={handleMediaAction}
                    />
                    <PerformanceTab
                      lang={lang}
                      sysinfo={sysinfo}
                      telemetry={telemetry}
                      networkHistory={networkHistory}
                    />
                    <WeatherTab
                      lang={lang}
                      weatherEnabled={settings.weatherEnabled}
                      weather={weather}
                      weatherError={weatherError}
                      onRefresh={() => refreshWeather(true)}
                    />
                  </>
                )}
              </div>
            </div>
          </div>
        </section>
        <div className="flare end" />
      </div>
    </div>
  );
}
