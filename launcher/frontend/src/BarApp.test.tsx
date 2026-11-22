import { act, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { BarApp, type BarBridge, type ShellSnapshot } from './BarApp';

const baseSnapshot: ShellSnapshot = {
  revision: 5,
  sessionGeneration: 2,
  topologyRevision: 3,
  helperConnected: true,
  syntheticBarsEnabled: true,
  barEdge: 'left',
  barThicknessLogical: 48,
  taskCount: 7,
  lastRecovery: 'startup',
  monitors: [
    {
      windowLabel: 'bar-display4',
      covered: false,
      barPlacement: { x: -1920, y: 540, width: 60, height: 1080 },
      monitor: {
        id: '\\\\.\\DISPLAY4',
        name: '\\\\.\\DISPLAY4',
        bounds: { left: -1920, top: 540, right: 0, bottom: 1620 },
        workArea: { left: -1920, top: 540, right: 0, bottom: 1620 },
        scaleFactor: 1.25,
        primary: false,
        fullscreen: false,
      },
    },
  ],
};

describe('BarApp (Milestone 2 synthetic bar)', () => {
  it('renders monitor projection and ignores stale revision snapshots', async () => {
    const handlers: Array<(snap: ShellSnapshot) => void> = [];
    const bridge: BarBridge = {
      getSnapshot: vi.fn().mockResolvedValue(baseSnapshot),
      onSnapshot: vi.fn().mockImplementation(async handler => {
        handlers.push(handler);
        return () => {};
      }),
    };

    render(<BarApp monitorLabel="bar-display4" bridge={bridge} />);

    expect(await screen.findByTestId('bar-monitor-id')).toHaveTextContent('DISPLAY4');
    expect(screen.getByTestId('bar-task-count')).toHaveTextContent('7t');
    expect(screen.getByTestId('bar-gen')).toHaveTextContent('g2');

    // Stale revision must be ignored.
    act(() => {
      handlers.forEach(fn => fn({ ...baseSnapshot, revision: 2, taskCount: 99 }));
    });
    expect(screen.getByTestId('bar-task-count')).toHaveTextContent('7t');
  });
});
