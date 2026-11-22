import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { DashboardBridge } from './api/tauriBridge';
import { DashboardApp } from './DashboardApp';
import { useDashboardStore } from './store/dashboardStore';
import type { DashboardSnapshot, VisibilityEvent } from './types/ipc';

function createSnapshot(open: boolean): DashboardSnapshot {
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
      language: 'pt-BR',
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
    updateSettings: vi.fn().mockResolvedValue(snapshot.settings),
    onVisibility: vi.fn().mockImplementation(async handler => {
      visibilityHandler = handler;
      return () => {
        visibilityHandler = null;
      };
    }),
    onSettings: vi.fn().mockResolvedValue(() => {}),
    emitVisibility: event => visibilityHandler?.(event),
  };
}

describe('DashboardApp', () => {
  beforeEach(() => {
    useDashboardStore.getState().reset();
  });

  it('does not mount active tab content while hidden', async () => {
    const bridge = createMockBridge(createSnapshot(false));
    render(<DashboardApp bridge={bridge} />);

    await waitFor(() => expect(bridge.getSnapshot).toHaveBeenCalledTimes(1));
    expect(screen.queryByTestId('tab-panel-dash')).not.toBeInTheDocument();
  });

  it('renders active tab when open and ignores stale revision events', async () => {
    const bridge = createMockBridge(createSnapshot(true));
    render(<DashboardApp bridge={bridge} />);

    await screen.findByTestId('tab-panel-dash');

    // Out-of-order event with lower revision must be ignored.
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

    fireEvent.click(screen.getByRole('tab', { name: 'Performance' }));
    expect(bridge.setActiveTab).toHaveBeenCalledWith('performance');
    await screen.findByTestId('tab-panel-performance');
  });

  it('closes dashboard on Escape when visible', async () => {
    const bridge = createMockBridge(createSnapshot(true));
    render(<DashboardApp bridge={bridge} />);
    await screen.findByTestId('tab-panel-dash');

    fireEvent.keyDown(window, { key: 'Escape' });
    expect(bridge.setVisibility).toHaveBeenCalledWith(false, 'DISPLAY1', 'hotkey');
  });
});
