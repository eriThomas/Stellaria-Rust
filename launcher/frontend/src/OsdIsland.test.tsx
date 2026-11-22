import { act, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const listeners = new Map<string, (ev: { payload: unknown }) => void>();

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(async () => ({ settings: { themeMode: 'dark', scheme: 'tonal', accent: '#6750a4', language: 'en' } })),
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(async (event: string, cb: (ev: { payload: unknown }) => void) => {
    listeners.set(event, cb);
    return () => listeners.delete(event);
  }),
}));

import { IslandApp } from './IslandApp';
import { OsdApp } from './OsdApp';

describe('OsdApp & IslandApp (Milestone 6)', () => {
  beforeEach(() => {
    listeners.clear();
  });

  it('renders OSD volume fill and muted state accurately', async () => {
    const { container } = render(<OsdApp />);
    await act(async () => {
      listeners.get('osd://show')?.({ payload: { kind: 'volume', value: 80, muted: false } });
    });
    const fill = container.querySelector('.vslider-fill') as HTMLElement;
    expect(fill.style.height).toBe('80%');
    expect(container.querySelector('.vslider')?.classList.contains('muted')).toBe(false);

    await act(async () => {
      listeners.get('osd://show')?.({ payload: { kind: 'volume', value: 80, muted: true } });
    });
    expect(fill.style.height).toBe('0%');
    expect(container.querySelector('.vslider')?.classList.contains('muted')).toBe(true);
  });

  it('renders Island toast label, title, and subtitle', async () => {
    render(<IslandApp />);
    await act(async () => {
      listeners.get('island://show')?.({
        payload: {
          label: 'Keyboard layout',
          title: 'Portuguese (Brazil)',
          subtitle: 'pt-BR',
          icon: 'keyboard',
        },
      });
    });
    expect(await screen.findByText('Keyboard layout')).toBeTruthy();
    expect(screen.getByText('Portuguese (Brazil)')).toBeTruthy();
    expect(screen.getByText('pt-BR')).toBeTruthy();
  });
});
