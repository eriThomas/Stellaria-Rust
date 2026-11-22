import { useState } from 'react';
import { formatBytes, formatGiB, translate, type SupportedLanguage } from '../i18n/messages';
import type { SystemInfo, TelemetrySnapshot } from '../types/ipc';
import { MetricRing } from './MetricRing';

interface PerformanceTabProps {
  lang: SupportedLanguage;
  sysinfo: SystemInfo | null;
  telemetry: TelemetrySnapshot | null;
  networkHistory: { down: number[]; up: number[] };
}

function buildSparklinePath(values: number[], max: number, width: number, height: number): string {
  if (values.length < 2) return '';
  const step = width / Math.max(1, values.length - 1);
  return values
    .map((v, i) => {
      const x = i * step;
      const y = height - (Math.min(v, max) / max) * (height - 4) - 2;
      return `${i === 0 ? 'M' : 'L'}${x.toFixed(1)} ${y.toFixed(1)}`;
    })
    .join('');
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

  const cpuPercent = telemetry?.cpuPercent ?? 0;
  const gpuUsage = telemetry?.gpu?.usagePercent ?? null;
  const gpuTemp = telemetry?.gpu?.temperatureC ?? null;

  const netMax = Math.max(1024, ...networkHistory.down, ...networkHistory.up);
  const downPath = buildSparklinePath(networkHistory.down, netMax, 240, 64);
  const upPath = buildSparklinePath(networkHistory.up, netMax, 240, 64);

  return (
    <section data-testid="tab-panel-performance">
      <div className="perf-grid">
        <article className="card">
          <strong>{translate(lang, 'cpu')}</strong>
          <small>{sysinfo?.cpu || '—'}</small>
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
            <span>{translate(lang, 'usage')}</span>
            <strong>{Math.round(cpuPercent)}%</strong>
          </div>
          <div className="progress-track">
            <div className="progress-fill" style={{ width: `${Math.min(100, Math.round(cpuPercent))}%` }} />
          </div>
        </article>

        <article className="card">
          <strong>{translate(lang, 'gpu')}</strong>
          <small>{telemetry?.gpu?.name || sysinfo?.gpu || '—'}</small>
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
            <span>{gpuTemp !== null ? `${Math.round(gpuTemp)} °C` : translate(lang, 'temp_unavailable')}</span>
            <strong>{gpuUsage !== null ? `${Math.round(gpuUsage)}%` : '—'}</strong>
          </div>
          <div className="progress-track">
            <div
              className="progress-fill"
              style={{
                width: `${gpuUsage !== null ? Math.min(100, Math.round(gpuUsage)) : 0}%`,
                background: 'var(--m3-secondary)',
              }}
            />
          </div>
        </article>
      </div>

      <div className="perf-cards-row">
        <article className="card" style={{ alignItems: 'center' }}>
          <MetricRing
            fraction={diskFraction}
            size={104}
            stroke={8}
            color="var(--m3-secondary)"
            label={translate(lang, 'storage')}
          />
          <small>
            {activeDisk
              ? `${formatGiB(diskUsed)} / ${formatGiB(activeDisk.totalBytes)} GiB`
              : '—'}
          </small>
          {disks.length > 0 && (
            <button
              type="button"
              className="icon-btn"
              onClick={() => setSelectedDiskIndex(i => (i + 1) % disks.length)}
            >
              {activeDisk?.mountPoint}
            </button>
          )}
        </article>

        <article className="card">
          <strong>{translate(lang, 'network')}</strong>
          <svg width="100%" height="64" viewBox="0 0 240 64" preserveAspectRatio="none">
            {upPath && <path d={upPath} fill="none" stroke="var(--m3-secondary)" strokeWidth="2" />}
            {downPath && <path d={downPath} fill="none" stroke="var(--m3-tertiary)" strokeWidth="2" />}
          </svg>
          <small>
            {translate(lang, 'download')}: {formatBytes(telemetry?.network.rxBytesPerSec ?? 0, true)}
          </small>
          <small>
            {translate(lang, 'upload')}: {formatBytes(telemetry?.network.txBytesPerSec ?? 0, true)}
          </small>
        </article>

        <article className="card" style={{ alignItems: 'center' }}>
          <MetricRing
            fraction={memFraction}
            size={104}
            stroke={8}
            color="var(--m3-tertiary)"
            label={translate(lang, 'memory')}
          />
          <small>
            {telemetry
              ? `${formatGiB(telemetry.memoryUsedBytes)} / ${formatGiB(telemetry.memoryTotalBytes)} GiB`
              : '—'}
          </small>
        </article>
      </div>
    </section>
  );
}
