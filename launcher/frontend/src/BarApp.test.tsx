import { act, fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { BarApp, type BarBridge, type ShellSnapshot, type TaskbarStatePayload } from './BarApp';

const baseSnapshot: ShellSnapshot = {
  revision: 5,
  sessionGeneration: 2,
  topologyRevision: 3,
  helperConnected: true,
  syntheticBarsEnabled: true,
  barEdge: 'left',
  barThicknessLogical: 48,
  taskCount: 2,
  lastRecovery: 'startup',
  monitors: [
    {
      windowLabel: 'bar-display1',
      covered: false,
      barPlacement: { x: 0, y: 0, width: 48, height: 1080 },
      monitor: {
        id: '\\\\.\\DISPLAY1',
        name: '\\\\.\\DISPLAY1',
        bounds: { left: 0, top: 0, right: 1920, bottom: 1080 },
        workArea: { left: 0, top: 0, right: 1920, bottom: 1080 },
        scaleFactor: 1.0,
        primary: true,
        fullscreen: false,
      },
    },
  ],
};

const baseTasks: TaskbarStatePayload = {
  revision: 1,
  sessionGeneration: 2,
  icons: {},
  groups: [
    {
      key: 'vivaldi.exe',
      displayName: 'Vivaldi',
      exePath: 'C:\\Apps\\vivaldi.exe',
      pinned: true,
      active: true,
      windowCount: 4,
      windows: [
        {
          hwnd: 101,
          pid: 1001,
          title: 'Stellaria',
          exePath: 'C:\\Apps\\vivaldi.exe',
          exeName: 'vivaldi.exe',
          monitorId: '\\\\.\\DISPLAY1',
          minimized: false,
          active: true,
        },
      ],
    },
  ],
};

describe('BarApp (Rust Taskbar)', () => {
  it('renders grouped tasks, activates on click, and opens context menu on right-click', async () => {
    const handlers: Array<(snap: ShellSnapshot) => void> = [];
    const showPreview = vi.fn().mockResolvedValue(undefined);
    const bridge: BarBridge = {
      getSnapshot: vi.fn().mockResolvedValue(baseSnapshot),
      getTaskbarState: vi.fn().mockResolvedValue(baseTasks),
      onSnapshot: vi.fn().mockImplementation(async h => {
        handlers.push(h);
        return () => {};
      }),
      onTasks: vi.fn().mockResolvedValue(() => {}),
      activateWindow: vi.fn().mockResolvedValue(undefined),
      closeWindow: vi.fn().mockResolvedValue(undefined),
      togglePin: vi.fn().mockResolvedValue(false),
      launchApp: vi.fn().mockResolvedValue(undefined),
      toggleLauncher: vi.fn().mockResolvedValue(undefined),
      openDashboard: vi.fn().mockResolvedValue(undefined),
      showPreview,
    };

    render(<BarApp monitorLabel="bar-display1" bridge={bridge} />);

    const vivaldiBtn = await screen.findByTitle('Vivaldi (4)');
    fireEvent.click(vivaldiBtn);
    expect(bridge.activateWindow).toHaveBeenCalledWith(101, 1001, false);

    fireEvent.contextMenu(vivaldiBtn);
    expect(showPreview).toHaveBeenCalledWith('\\\\.\\DISPLAY1', 'vivaldi.exe', expect.any(Number), 'context');

    act(() => {
      handlers.forEach(fn => fn({ ...baseSnapshot, revision: 2 }));
    });
  });
});
