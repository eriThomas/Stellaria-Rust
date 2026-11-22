import { useState } from 'react';
import { formatBytes, formatGiB, translate, type SupportedLanguage } from '../i18n/messages';
import type { SystemInfo, TelemetrySnapshot } from '../types/ipc';
import { ArcRing, ExpressiveShape, Icon, type ShapeName } from './ExpressiveShape';

interface PerformanceTabProps {
  lang: SupportedLanguage;
  sysinfo: SystemInfo | null;
  telemetry: TelemetrySnapshot | null;
  networkHistory: { down: number[]; up: number[] };
}

function shapeForUsage(percent: number | null): ShapeName {
  if (percent === null || !Number.isFinite(percent) || percent < 40) return 'cookie4';
  if (percent < 80) return 'sunny';
  return 'softBurst';
}

function sparklinePaths(values: number[], max: number, width: number, height: number): { line: string; area: string } {
  if (values.length < 2) return { line: '', area: '' };
  const step = width / Math.max(1, 38);
  const offset = (40 - values.length) * step;
  const points = values.map((v, i) => [
    offset + i * step,
    height - (Math.min(v, max) / max) * (height - 4) - 2,
  ]);
  const line = points.map(([x, y], i) => `${i ? 'L' : 'M'}${x.toFixed(1)} ${y.toFixed(1)}`).join('');
  const area = `${line}L${points[points.length - 1][0].toFixed(1)} ${height}L${points[0][0].toFixed(1)} ${height}Z`;
  return { line, area };
}

export function PerformanceTab({
  lang,
  sysinfo,
  telemetry,
  networkHistory,
}: PerformanceTabProps) {
  const [selectedDiskIndex, setSelectedDiskIndex] = useState(0);

  const disks = telemetry?.disks ?? [];
  const activeDisk = disks.length > 0 ? disks[selectedDiskIndex % disks.length] : null;
  const diskUsed = activeDisk ? activeDisk.totalBytes - activeDisk.availableBytes : 0;
  const diskFraction = activeDisk && activeDisk.totalBytes > 0 ? diskUsed / activeDisk.totalBytes : 0;

  const memFraction =
    telemetry && telemetry.memoryTotalBytes > 0
      ? telemetry.memoryUsedBytes / telemetry.memoryTotalBytes
      : 0;

  const cpuUsage = telemetry ? telemetry.cpuPercent : null;
  const gpuUsage = telemetry?.gpu?.usagePercent ?? null;
  const gpuTemp = telemetry?.gpu?.temperatureC ?? null;

  const netMax = Math.max(1024, ...networkHistory.down, ...networkHistory.up);
  const up = sparklinePaths(networkHistory.up, netMax, 358, 90);
  const down = sparklinePaths(networkHistory.down, netMax, 358, 90);

  return (
    <section className="page performance" data-page="performance" data-testid="tab-panel-performance">
      <div className="perf-row heroes">
        <article className="card perf-hero" style={{ '--stagger': 0 } as React.CSSProperties}>
          <ArcRing
            size={52}
            stroke={4}
            color="var(--m3-primary)"
            fraction={(cpuUsage ?? 0) / 100}
            className="perf-hero-ring"
          >
            <Icon name="memory" style={{ color: 'var(--m3-primary)' }} />
          </ArcRing>
          <div className="perf-hero-title">
            <span className="perf-hero-label" style={{ color: 'var(--m3-primary)' }}>
              {translate(lang, 'cpu')}
            </span>
            <span className="perf-hero-sub" title={sysinfo?.cpu || '—'}>
              {sysinfo?.cpu || '—'}
            </span>
          </div>
          <div className="perf-hero-temp">
            <div className="perf-hero-temp-row">
              <Icon name="thermometer" style={{ color: 'var(--m3-primary)' }} />
              <span>{translate(lang, 'temp_unavailable')}</span>
            </div>
            <div className="bar-track indeterminate">
              <div className="bar-fill" style={{ background: 'var(--m3-primary)', width: '35%' }} />
            </div>
          </div>
          <div className="perf-hero-usage">
            <span className="perf-hero-usage-label">{translate(lang, 'usage')}</span>
            <ExpressiveShape name={shapeForUsage(cpuUsage)} />
            <span className="perf-hero-usage-value" style={{ color: 'var(--m3-primary)' }}>
              {cpuUsage !== null ? `${Math.round(cpuUsage)}%` : '…'}
            </span>
          </div>
        </article>

        <article className="card perf-hero" style={{ '--stagger': 1 } as React.CSSProperties}>
          <ArcRing
            size={52}
            stroke={4}
            color="var(--m3-secondary)"
            fraction={(gpuUsage ?? 0) / 100}
            className="perf-hero-ring"
          >
            <Icon name="desktop_windows" style={{ color: 'var(--m3-secondary)' }} />
          </ArcRing>
          <div className="perf-hero-title">
            <span className="perf-hero-label" style={{ color: 'var(--m3-secondary)' }}>
              {translate(lang, 'gpu')}
            </span>
            <span className="perf-hero-sub" title={telemetry?.gpu?.name || sysinfo?.gpu || '—'}>
              {telemetry?.gpu?.name || sysinfo?.gpu || '—'}
            </span>
          </div>
          <div className="perf-hero-temp">
            <div className="perf-hero-temp-row">
              <Icon
                name={gpuTemp !== null && gpuTemp > 90 ? 'thermometer_alert' : 'thermometer'}
                style={{ color: gpuTemp !== null && gpuTemp > 90 ? 'var(--m3-error)' : 'var(--m3-secondary)' }}
              />
              <span>{gpuTemp !== null ? `${Math.round(gpuTemp)} °C` : translate(lang, 'temp_unavailable')}</span>
            </div>
            <div className={`bar-track${gpuTemp === null ? ' indeterminate' : ''}`}>
              <div
                className="bar-fill"
                style={{
                  background: 'var(--m3-secondary)',
                  width: `${gpuTemp !== null ? Math.min(100, Math.round(gpuTemp)) : 35}%`,
                }}
              />
            </div>
          </div>
          <div className="perf-hero-usage">
            <span className="perf-hero-usage-label">{translate(lang, 'usage')}</span>
            <ExpressiveShape name={shapeForUsage(gpuUsage)} />
            <span className="perf-hero-usage-value" style={{ color: 'var(--m3-secondary)' }}>
              {gpuUsage !== null ? `${Math.round(gpuUsage)}%` : '…'}
            </span>
          </div>
        </article>
      </div>

      <div className="perf-row">
        <article className="card storage" style={{ '--stagger': 2 } as React.CSSProperties}>
          <div className="storage-row">
            <ArcRing
              size={124}
              stroke={8}
              start={135}
              sweep={270}
              color="var(--m3-secondary)"
              fraction={diskFraction}
              className="big-ring"
            >
              <div className="big-ring-content">
                <Icon name="hard_drive" className="secondary" />
                <span className="big-ring-value" style={{ color: 'var(--m3-secondary)' }}>
                  {activeDisk ? `${Math.round(diskFraction * 100)}%` : '—'}
                </span>
                <span className="big-ring-label">{translate(lang, 'used')}</span>
              </div>
            </ArcRing>
            <div className="storage-text">
              <span className="storage-name">{translate(lang, 'storage')}</span>
              <span className="storage-usage">
                {activeDisk ? `${formatGiB(diskUsed)} / ${formatGiB(activeDisk.totalBytes)} GiB` : '—'}
              </span>
            </div>
          </div>
          <button
            type="button"
            className="disk-button state-layer"
            onClick={() => disks.length > 1 && setSelectedDiskIndex(i => (i + 1) % disks.length)}
          >
            <Icon name="storage" />
            <span>{activeDisk?.mountPoint ?? 'C:\\'}</span>
            <Icon name="unfold_more" />
          </button>
        </article>

        <article className="card network" style={{ '--stagger': 3 } as React.CSSProperties}>
          <div className="perf-card-header">
            <Icon name="swap_vert" className="primary" />
            <span>{translate(lang, 'network')}</span>
          </div>
          <div className="network-graph">
            <svg viewBox="0 0 358 90" preserveAspectRatio="none">
              {up.area && <path d={up.area} fill="var(--m3-secondary)" fillOpacity="0.15" />}
              {up.line && <path d={up.line} className="line" stroke="var(--m3-secondary)" />}
              {down.area && <path d={down.area} fill="var(--m3-tertiary)" fillOpacity="0.2" />}
              {down.line && <path d={down.line} className="line" stroke="var(--m3-tertiary)" />}
            </svg>
            <span className="collecting" hidden={networkHistory.down.length >= 2}>
              {translate(lang, 'loading')}
            </span>
          </div>
          <div className="network-row">
            <Icon name="download" className="tertiary" />
            <span>{translate(lang, 'download')}</span>
            <span className="value" style={{ color: 'var(--m3-tertiary)' }}>
              {formatBytes(telemetry?.network.rxBytesPerSec ?? 0, true)}
            </span>
          </div>
          <div className="network-row">
            <Icon name="upload" className="secondary" />
            <span>{translate(lang, 'upload')}</span>
            <span className="value" style={{ color: 'var(--m3-secondary)' }}>
              {formatBytes(telemetry?.network.txBytesPerSec ?? 0, true)}
            </span>
          </div>
          <div className="network-row">
            <Icon name="history" />
            <span>{translate(lang, 'total')}</span>
            <span className="value" style={{ fontSize: '12.5px', fontWeight: 400 }}>
              ↓{formatBytes(telemetry?.network.totalRxBytes ?? 0)} ↑
              {formatBytes(telemetry?.network.totalTxBytes ?? 0)}
            </span>
          </div>
        </article>

        <article className="card memory" style={{ '--stagger': 4 } as React.CSSProperties}>
          <div className="perf-card-header">
            <Icon name="memory_alt" filled className="tertiary" />
            <span>{translate(lang, 'memory')}</span>
          </div>
          <ArcRing
            size={124}
            stroke={8}
            start={135}
            sweep={270}
            color="var(--m3-tertiary)"
            fraction={memFraction}
            className="big-ring"
          >
            <div className="big-ring-content">
              <span className="big-ring-value" style={{ color: 'var(--m3-tertiary)' }}>
                {telemetry ? `${Math.round(memFraction * 100)}%` : '—'}
              </span>
              <span className="big-ring-label">{translate(lang, 'used')}</span>
            </div>
          </ArcRing>
          <span className="memory-usage">
            {telemetry
              ? `${formatGiB(telemetry.memoryUsedBytes)} / ${formatGiB(telemetry.memoryTotalBytes)} GiB`
              : '—'}
          </span>
        </article>
      </div>
    </section>
  );
}
