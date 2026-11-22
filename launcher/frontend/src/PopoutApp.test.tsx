import { act, fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const invokeMock = vi.fn();
const listeners = new Map<string, (ev: { payload: unknown }) => void>();

vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(async (event: string, cb: (ev: { payload: unknown }) => void) => {
    listeners.set(event, cb);
    return () => listeners.delete(event);
  }),
}));

import { PopoutApp } from './PopoutApp';

describe('PopoutApp (Milestone 6 & 7)', () => {
  beforeEach(() => {
    invokeMock.mockReset();
    listeners.clear();
    localStorage.clear();
    invokeMock.mockImplementation(async (cmd: string) => {
      switch (cmd) {
        case 'dashboard_get_snapshot':
          return { settings: { themeMode: 'dark', scheme: 'tonal', accent: '#6750a4', language: 'en' } };
        case 'shell_get_master_audio':
          return { volume: 72, muted: false };
        case 'popout_get_audio_sessions':
          return [{ id: 's1', name: 'Spotify', system: false, volume: 65, muted: false, icon: null }];
        case 'popout_get_network_info':
          return {
            connected: true,
            adapter: 'Realtek PCIe GbE',
            adapterType: 'ethernet',
            ipv4: '192.168.0.83',
            mac: 'AA:BB:CC:DD:EE:FF',
            linkSpeedMbps: 1000,
            rxBytesPerSec: 2048,
            txBytesPerSec: 1024,
          };
        case 'popout_get_keyboard_layouts':
          return {
            current: '00000409',
            items: [
              { id: '00000409', code: 'US', name: 'English (United States)', language: 'en-US' },
              { id: '00000416', code: 'PT', name: 'Portuguese (Brazil)', language: 'pt-BR' },
            ],
          };
        case 'popout_get_notifications':
          return [{ id: 'n1', appId: 'Vivaldi', title: 'Build done', body: 'All green', time: Date.now() }];
        default:
          return undefined;
      }
    });
  });

  it('switches cleanly between audio, network, keyboard, and notifications views', async () => {
    render(<PopoutApp />);

    // Trigger audio view
    await act(async () => {
      listeners.get('popout://show')?.({ payload: { kind: 'audio' } });
    });
    expect(await screen.findByText('72%')).toBeTruthy();
    expect(await screen.findByText('Spotify')).toBeTruthy();

    // Switch to network view
    await act(async () => {
      listeners.get('popout://show')?.({ payload: { kind: 'network' } });
    });
    expect(await screen.findByText('Realtek PCIe GbE')).toBeTruthy();
    expect(screen.getByText('192.168.0.83')).toBeTruthy();
    expect(screen.getByText('AA:BB:CC:DD:EE:FF')).toBeTruthy();
    expect(screen.getByText('1 Gb/s')).toBeTruthy();

    // Switch to keyboard view and select PT layout
    await act(async () => {
      listeners.get('popout://show')?.({ payload: { kind: 'keyboard' } });
    });
    const ptBtn = await screen.findByText('Portuguese (Brazil)');
    fireEvent.click(ptBtn);
    expect(invokeMock).toHaveBeenCalledWith('popout_set_keyboard_layout', { id: '00000416' });
    expect(invokeMock).toHaveBeenCalledWith(
      'island_show_toast',
      expect.objectContaining({ label: 'Keyboard layout', title: 'Portuguese (Brazil)' }),
    );

    // Switch to notifications view and dismiss item
    await act(async () => {
      listeners.get('popout://show')?.({ payload: { kind: 'notifications' } });
    });
    expect(await screen.findByText('Build done')).toBeTruthy();
    fireEvent.click(screen.getByTitle('Dismiss'));
    expect(await screen.findByText('No notifications right now')).toBeTruthy();
  });
});
