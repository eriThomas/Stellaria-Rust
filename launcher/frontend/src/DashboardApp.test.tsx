import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { DashboardBridge } from './api/tauriBridge';
import { DashboardApp } from './DashboardApp';
import { useDashboardStore } from './store/dashboardStore';
import type { DashboardSnapshot, VisibilityEvent } from './types/ipc';

function createSnapshot(open: boolean, language = 'en'): DashboardSnapshot {
  return {
    revision: 2,
    sessionGeneration: 100,
    visibility: {
      open,
      monitorId: 'DISPLAY1',
      trigger: open ? 'tray' : null,
      suppressed: false,
      activeTab: 'dash',
    },
    settings: {
      schemaVersion: 1,
      language,
      clockSeconds: false,
      lyricsEnabled: true,
      weatherEnabled: true,
      themeMode: 'dark',
      themeVariant: 'tonalspot',
      sourceColor: '#7e57c2',
      reducedMotion: false,
      gameMode: false,
      borderSize: 0,
    },
    monitors: [],
    flags: {
      identity: 'experimental',
      dashboardPilot: true,
      zebarCoexistence: true,
    },
  };
}

function createMockBridge(snapshot: DashboardSnapshot): DashboardBridge & {
  emitVisibility: (event: VisibilityEvent) => void;
} {
  let visibilityHandler: ((event: VisibilityEvent) => void) | null = null;
  return {
    getSnapshot: vi.fn().mockResolvedValue(snapshot),
    setVisibility: vi.fn().mockResolvedValue({
      revision: snapshot.revision + 1,
      visibility: { ...snapshot.visibility, open: false },
    }),
    setActiveTab: vi.fn().mockImplementation(async tab => ({
      revision: snapshot.revision + 1,
      visibility: { ...snapshot.visibility, activeTab: tab },
    })),
    updateSettings: vi.fn().mockImplementation(async patch => ({
      ...snapshot.settings,
      ...patch,
    })),
    getSysInfo: vi.fn().mockResolvedValue({
      os: 'Windows 11 Pro',
      cpu: 'AMD Ryzen 9',
      gpu: 'NVIDIA RTX',
      user: 'Erick',
      pictureBase64: null,
    }),
    getTelemetry: vi.fn().mockResolvedValue({
      revision: 1,
      uptimeMs: 3_660_000,
      cpuPercent: 24.5,
      memoryUsedBytes: 8 * 1024 ** 3,
      memoryTotalBytes: 32 * 1024 ** 3,
      disks: [{ mountPoint: 'C:\\', totalBytes: 500 * 1024 ** 3, availableBytes: 250 * 1024 ** 3 }],
      network: { rxBytesPerSec: 1200, txBytesPerSec: 400, totalRxBytes: 50000, totalTxBytes: 10000 },
      gpu: { name: 'NVIDIA RTX', usagePercent: 32, temperatureC: 54 },
    }),
    getMedia: vi.fn().mockResolvedValue({
      current: {
        sessionId: 'spotify',
        title: 'Instant Crush',
        artist: 'Daft Punk',
        album: 'Random Access Memories',
        isPlaying: true,
        positionSecs: 30,
        durationSecs: 337,
      },
      sessions: [],
      artworkBase64: null,
    }),
    sendMediaAction: vi.fn().mockResolvedValue(undefined),
    refreshWeather: vi.fn().mockResolvedValue({
      fetchedAtMs: Date.now(),
      city: 'São Paulo',
      current: {
        temperature: 26,
        feelsLike: 27,
        humidity: 60,
        wind: 14,
        code: 0,
        isDay: true,
        icon: 'sunny',
        conditionKey: 'clear_sky',
      },
      sunrise: '2026-11-22T05:30',
      sunset: '2026-11-22T18:45',
      daily: [{ date: '2026-11-22', code: 0, icon: 'sunny', max: 28, min: 19 }],
    }),
    getLyrics: vi.fn().mockResolvedValue({
      key: 'Daft Punk|Instant Crush',
      lines: [{ timeSecs: 10, text: 'I did not want to be the one to forget' }],
    }),
    onVisibility: vi.fn().mockImplementation(async handler => {
      visibilityHandler = handler;
      return () => {
        visibilityHandler = null;
      };
    }),
    onSettings: vi.fn().mockResolvedValue(() => {}),
    onTelemetry: vi.fn().mockResolvedValue(() => {}),
    emitVisibility: event => visibilityHandler?.(event),
  };
}

describe('DashboardApp (Milestone 1)', () => {
  beforeEach(() => {
    useDashboardStore.getState().reset();
  });

  it('does not mount tabs or fetch telemetry/weather while hidden', async () => {
    const bridge = createMockBridge(createSnapshot(false));
    render(<DashboardApp bridge={bridge} />);

    await waitFor(() => expect(bridge.getSnapshot).toHaveBeenCalledTimes(1));
    expect(screen.queryByTestId('tab-panel-dash')).not.toBeInTheDocument();
    expect(bridge.getTelemetry).not.toHaveBeenCalled();
    expect(bridge.refreshWeather).not.toHaveBeenCalled();
  });

  it('renders pt-BR localization, switches tabs, and ignores stale revisions', async () => {
    const bridge = createMockBridge(createSnapshot(true, 'pt-BR'));
    render(<DashboardApp bridge={bridge} />);

    await screen.findByTestId('tab-panel-dash');
    expect(screen.getByRole('tab', { name: 'Desempenho' })).toBeInTheDocument();
    expect(await screen.findByText('São Paulo')).toBeInTheDocument();

    bridge.emitVisibility({
      revision: 1,
      visibility: {
        open: false,
        monitorId: 'DISPLAY1',
        trigger: null,
        suppressed: true,
        activeTab: 'dash',
      },
    });
    expect(screen.getByTestId('tab-panel-dash')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('tab', { name: 'Desempenho' }));
    await screen.findByTestId('tab-panel-performance');
    expect(screen.getByText('NVIDIA RTX')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('tab', { name: 'Clima' }));
    await screen.findByTestId('tab-panel-weather');
    expect(screen.getByText('Previsão de 7 dias')).toBeInTheDocument();
  });

  it('opens settings popover and persists updates via typed command', async () => {
    const bridge = createMockBridge(createSnapshot(true, 'en'));
    render(<DashboardApp bridge={bridge} />);
    await screen.findByTestId('tab-panel-dash');

    fireEvent.click(screen.getByRole('button', { name: 'Settings' }));
    expect(screen.getByTestId('settings-popover')).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText('Language'), { target: { value: 'pt-BR' } });
    await waitFor(() => expect(bridge.updateSettings).toHaveBeenCalledWith({ language: 'pt-BR' }));
  });

  it('closes dashboard on Escape when visible', async () => {
    const bridge = createMockBridge(createSnapshot(true));
    render(<DashboardApp bridge={bridge} />);
    await screen.findByTestId('tab-panel-dash');

    fireEvent.keyDown(window, { key: 'Escape' });
    expect(bridge.setVisibility).toHaveBeenCalledWith(false, 'DISPLAY1', 'hotkey');
  });
});
