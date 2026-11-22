import { useEffect, useMemo, useState } from 'react';
import { formatUptime, translate, type SupportedLanguage } from '../i18n/messages';
import type {
  MediaAction,
  MediaSnapshot,
  SystemInfo,
  TelemetrySnapshot,
  WeatherSnapshot,
} from '../types/ipc';
import { CalendarCard } from './CalendarCard';
import { arcPath, ArcRing, ExpressiveShape, HeroOrnament, Icon } from './ExpressiveShape';

interface HomeTabProps {
  lang: SupportedLanguage;
  isOpen: boolean;
  clockSeconds: boolean;
  weatherEnabled: boolean;
  weather: WeatherSnapshot | null;
  weatherError: string | null;
  sysinfo: SystemInfo | null;
  telemetry: TelemetrySnapshot | null;
  media: MediaSnapshot | null;
  onMediaAction: (action: MediaAction) => void;
}

const ARC_LENGTH = 94 * Math.PI;

export function HomeTab({
  lang,
  isOpen,
  clockSeconds,
  weatherEnabled,
  weather,
  weatherError,
  sysinfo,
  telemetry,
  media,
  onMediaAction,
}: HomeTabProps) {
  const [now, setNow] = useState(() => new Date());

  useEffect(() => {
    if (!isOpen) return;
    let timer = 0;
    const schedule = () => {
      const current = new Date();
      setNow(current);
      const elapsed = (clockSeconds ? 0 : current.getSeconds() * 1000) + current.getMilliseconds();
      const delay = (clockSeconds ? 1000 : 60_000) - elapsed + 20;
      timer = window.setTimeout(schedule, delay);
    };
    schedule();
    return () => window.clearTimeout(timer);
  }, [isOpen, clockSeconds]);

  const cpuFraction = (telemetry?.cpuPercent ?? 0) / 100;
  const memFraction =
    telemetry && telemetry.memoryTotalBytes > 0
      ? telemetry.memoryUsedBytes / telemetry.memoryTotalBytes
      : 0;
  const primaryDisk =
    telemetry?.disks.find(d => d.mountPoint.toUpperCase().startsWith('C:')) ?? telemetry?.disks[0];
  const diskFraction =
    primaryDisk && primaryDisk.totalBytes > 0
      ? (primaryDisk.totalBytes - primaryDisk.availableBytes) / primaryDisk.totalBytes
      : 0;

  const currentMedia = media?.current ?? null;
  const mediaFraction =
    currentMedia && currentMedia.durationSecs > 0
      ? Math.min(1, Math.max(0, currentMedia.positionSecs / currentMedia.durationSecs))
      : currentMedia?.isPlaying
        ? 0.55
        : 0;
  const dashArcPath = useMemo(() => arcPath(100, 100, 94, 180, 180), []);

  return (
    <section className="page dash" data-page="dash" data-testid="tab-panel-dash">
      <article className="card dash-weather" style={{ '--stagger': 0 } as React.CSSProperties}>
        <HeroOrnament name="flower" />
        <Icon name={weather?.current.icon ?? 'cloud'} className="dash-weather-icon" />
        <div className="dash-weather-info">
          <span className="dash-weather-temp">
            {weather ? `${Math.round(weather.current.temperature)}°C` : '--°C'}
          </span>
          <span className="dash-weather-label">
            {!weatherEnabled
              ? translate(lang, 'weather_disabled')
              : weatherError
                ? translate(lang, 'weather_unavailable')
                : weather
                  ? translate(lang, weather.current.conditionKey)
                  : translate(lang, 'loading')}
          </span>
        </div>
      </article>

      <article className="card dash-user" style={{ '--stagger': 1 } as React.CSSProperties}>
        <div className="user-logo">
          <ExpressiveShape name="gem" />
          <span className="logo-mark">
            <svg viewBox="0 0 24 24" aria-hidden="true">
              <path d="M3 5.1 10.1 4.1v6.8H3zM11 4l10-1.4v8.3H11zM3 11.9h7.1v6.9L3 17.8zM11 11.9h10v8.4L11 18.9z" />
            </svg>
          </span>
        </div>
        <div className="user-picture">
          {sysinfo?.pictureBase64 ? (
            <img src={`data:image/png;base64,${sysinfo.pictureBase64}`} alt="" />
          ) : (
            <Icon name="person" filled />
          )}
        </div>
        <div className="user-wm">
          <Icon name="select_window" />
          <span>{sysinfo?.os ? `${sysinfo.os}…` : 'Windows…'}</span>
        </div>
        <span className="user-bubble small" />
        <span className="user-bubble large" />
        <div className="user-uptime">
          <ExpressiveShape name="clamShell" />
          <Icon name="clock_arrow_up" />
        </div>
        <span className="user-uptime-text">
          {telemetry ? formatUptime(lang, telemetry.uptimeMs) : '—'}
        </span>
      </article>

      <article className="card dash-time" style={{ '--stagger': 2 } as React.CSSProperties} data-testid="dashboard-clock">
        <HeroOrnament name="cookie4" />
        <span className="time-digits">{String(now.getHours()).padStart(2, '0')}</span>
        <span className="time-dots">•••</span>
        <span className="time-digits">{String(now.getMinutes()).padStart(2, '0')}</span>
        <span className="time-seconds" hidden={!clockSeconds}>
          {String(now.getSeconds()).padStart(2, '0')}
        </span>
      </article>

      <CalendarCard lang={lang} />

      <article className="card dash-resources" style={{ '--stagger': 4 } as React.CSSProperties}>
        <ArcRing
          size={61}
          stroke={6}
          color="var(--m3-primary)"
          fraction={cpuFraction}
          className="resource"
          title={`${translate(lang, 'cpu')}: ${Math.round(cpuFraction * 100)}%`}
        >
          <Icon name="memory" className="primary" />
        </ArcRing>
        <ArcRing
          size={61}
          stroke={6}
          color="var(--m3-tertiary)"
          fraction={memFraction}
          className="resource"
          title={`${translate(lang, 'memory')}: ${Math.round(memFraction * 100)}%`}
        >
          <Icon name="memory_alt" className="tertiary" />
        </ArcRing>
        <ArcRing
          size={61}
          stroke={6}
          color="var(--m3-secondary)"
          fraction={diskFraction}
          className="resource"
          title={`${translate(lang, 'storage')}: ${Math.round(diskFraction * 100)}%`}
        >
          <Icon name="hard_disk" className="secondary" />
        </ArcRing>
      </article>

      <article className="card dash-media" style={{ '--stagger': 5 } as React.CSSProperties}>
        <HeroOrnament name="softBurst" />
        <div className="dash-cover-wrap">
          <svg className="media-arc" viewBox="0 0 200 200">
            <path className="arc-track" d={dashArcPath} />
            <path
              className="arc-value"
              d={dashArcPath}
              strokeDasharray={`${(ARC_LENGTH * mediaFraction).toFixed(2)} ${(ARC_LENGTH + 1).toFixed(2)}`}
            />
          </svg>
          <div className="cover">
            {media?.artworkBase64 ? (
              <img src={`data:image/jpeg;base64,${media.artworkBase64}`} alt="" />
            ) : (
              <Icon name="art_track" filled />
            )}
          </div>
        </div>
        <span className="dash-media-title">
          {currentMedia ? currentMedia.title || translate(lang, 'no_media') : translate(lang, 'no_media')}
        </span>
        <span className="dash-media-album">
          {currentMedia ? currentMedia.album || translate(lang, 'unknown_album') : translate(lang, 'no_media')}
        </span>
        <span className="dash-media-artist">
          {currentMedia ? currentMedia.artist || translate(lang, 'unknown_artist') : translate(lang, 'no_media')}
        </span>
        <div className="button-row">
          <button
            type="button"
            className="icon-button state-layer"
            disabled={!currentMedia}
            onClick={() => onMediaAction('previous')}
          >
            <Icon name="skip_previous" filled />
          </button>
          <button
            type="button"
            className={`icon-button state-layer fill${currentMedia?.isPlaying ? ' checked' : ''}`}
            disabled={!currentMedia}
            onClick={() => onMediaAction('togglePlayPause')}
          >
            <Icon name={currentMedia?.isPlaying ? 'pause' : 'play_arrow'} filled />
          </button>
          <button
            type="button"
            className="icon-button state-layer"
            disabled={!currentMedia}
            onClick={() => onMediaAction('next')}
          >
            <Icon name="skip_next" filled />
          </button>
        </div>
      </article>
    </section>
  );
}
