import { useEffect, useState } from 'react';
import { formatUptime, translate, type SupportedLanguage } from '../i18n/messages';
import type {
  MediaAction,
  MediaSnapshot,
  SystemInfo,
  TelemetrySnapshot,
  WeatherSnapshot,
} from '../types/ipc';
import { CalendarCard } from './CalendarCard';
import { MetricRing } from './MetricRing';

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

  return (
    <section className="home-grid" data-testid="tab-panel-dash">
      <article className="card">
        <strong>{translate(lang, 'tab_weather')}</strong>
        {!weatherEnabled ? (
          <span>{translate(lang, 'weather_disabled')}</span>
        ) : weatherError ? (
          <span>{translate(lang, 'weather_unavailable')}</span>
        ) : weather ? (
          <>
            <span style={{ fontSize: 28, fontWeight: 700 }}>
              {Math.round(weather.current.temperature)}°C
            </span>
            <span>{translate(lang, weather.current.conditionKey)}</span>
            <small>{weather.city}</small>
          </>
        ) : (
          <span>{translate(lang, 'loading')}</span>
        )}
      </article>

      <article className="card">
        <strong>{sysinfo?.user || 'Stellaria'}</strong>
        <span>{sysinfo?.os || 'Windows 11'}</span>
        <small>{telemetry ? formatUptime(lang, telemetry.uptimeMs) : '—'}</small>
      </article>

      <article className="card">
        <span className="clock-digits" data-testid="dashboard-clock">
          {String(now.getHours()).padStart(2, '0')}:{String(now.getMinutes()).padStart(2, '0')}
          {clockSeconds ? `:${String(now.getSeconds()).padStart(2, '0')}` : ''}
        </span>
      </article>

      <CalendarCard lang={lang} />

      <article className="card">
        <div className="rings-row">
          <MetricRing fraction={cpuFraction} color="var(--m3-primary)" label={translate(lang, 'cpu')} />
          <MetricRing fraction={memFraction} color="var(--m3-tertiary)" label={translate(lang, 'memory')} />
          <MetricRing fraction={diskFraction} color="var(--m3-secondary)" label={translate(lang, 'storage')} />
        </div>
      </article>

      <article className="card">
        <strong>{currentMedia?.title || translate(lang, 'no_media')}</strong>
        <small>{currentMedia?.artist || translate(lang, 'no_media')}</small>
        <div className="controls-row">
          <button
            type="button"
            className="icon-btn"
            disabled={!currentMedia}
            onClick={() => onMediaAction('previous')}
          >
            ⏮
          </button>
          <button
            type="button"
            className="icon-btn"
            disabled={!currentMedia}
            onClick={() => onMediaAction('togglePlayPause')}
          >
            {currentMedia?.isPlaying ? '⏸' : '▶'}
          </button>
          <button
            type="button"
            className="icon-btn"
            disabled={!currentMedia}
            onClick={() => onMediaAction('next')}
          >
            ⏭
          </button>
        </div>
      </article>
    </section>
  );
}
