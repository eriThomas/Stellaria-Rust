import { act, fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { PreviewApp, type PreviewBridge, type PreviewPayload } from './PreviewApp';

const samplePayload: PreviewPayload = {
  iconUrl: null,
  group: {
    key: 'vivaldi.exe',
    displayName: 'Vivaldi',
    exePath: 'C:\\Apps\\vivaldi.exe',
    pinned: true,
    active: true,
    windowCount: 2,
    windows: [
      {
        hwnd: 201,
        pid: 4001,
        title: 'GitHub - Stellaria',
        exePath: 'C:\\Apps\\vivaldi.exe',
        exeName: 'vivaldi.exe',
        monitorId: '\\\\.\\DISPLAY1',
        minimized: false,
        active: true,
      },
      {
        hwnd: 202,
        pid: 4001,
        title: 'Minimized Docs',
        exePath: 'C:\\Apps\\vivaldi.exe',
        exeName: 'vivaldi.exe',
        monitorId: '\\\\.\\DISPLAY2',
        minimized: true,
        active: false,
      },
    ],
  },
};

describe('PreviewApp (Milestone 4)', () => {
  it('activates clicked window and closes individual window without blocking minimized fallback', async () => {
    const handlers: Array<(p: PreviewPayload) => void> = [];
    const bridge: PreviewBridge = {
      onShow: vi.fn().mockImplementation(async h => {
        handlers.push(h);
        return () => {};
      }),
      capturePreview: vi.fn().mockImplementation(async (hwnd, pid) => ({
        identity: { hwnd, pid, sessionGeneration: 1 },
        width: 280,
        height: 140,
        pngBase64: hwnd === 201 ? 'data:image/bmp;base64,AAA' : null,
        capturedAtMs: Date.now(),
        fallbackReason: hwnd === 202 ? 'minimized' : null,
      })),
      activateWindow: vi.fn().mockResolvedValue(undefined),
      closeWindow: vi.fn().mockResolvedValue(undefined),
      hidePreview: vi.fn().mockResolvedValue(undefined),
    };

    render(<PreviewApp bridge={bridge} />);
    act(() => {
      handlers.forEach(h => h(samplePayload));
    });

    expect(await screen.findByTestId('preview-card-201')).toBeInTheDocument();
    expect(screen.getByTestId('preview-card-202')).toBeInTheDocument();

    // Minimized window remains clickable
    fireEvent.click(screen.getByRole('button', { name: 'Activate Minimized Docs' }));
    expect(bridge.activateWindow).toHaveBeenCalledWith(202, 4001);
    expect(bridge.hidePreview).toHaveBeenCalled();

    // Close single window removes only that card
    fireEvent.click(screen.getByRole('button', { name: 'Close GitHub - Stellaria' }));
    expect(bridge.closeWindow).toHaveBeenCalledWith(201, 4001);
    expect(screen.queryByTestId('preview-card-201')).not.toBeInTheDocument();
    expect(screen.getByTestId('preview-card-202')).toBeInTheDocument();
  });
});
