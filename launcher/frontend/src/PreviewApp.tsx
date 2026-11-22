import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { useEffect, useState } from 'react';
import type { TaskGroup } from './BarApp';
import { Icon } from './components/ExpressiveShape';

export interface PreviewEntry {
  identity: { hwnd: number; pid: number; sessionGeneration: number };
  width: number;
  height: number;
  pngBase64: string | null;
  capturedAtMs: number;
  fallbackReason: string | null;
}

export interface PreviewPayload {
  group: TaskGroup;
  iconUrl: string | null;
  mode?: 'preview' | 'context';
}

export interface PreviewBridge {
  onShow: (handler: (payload: PreviewPayload) => void) => Promise<UnlistenFn>;
  capturePreview: (hwnd: number, pid: number) => Promise<PreviewEntry>;
  activateWindow: (hwnd: number, pid: number) => Promise<void>;
  closeWindow: (hwnd: number, pid: number) => Promise<void>;
  togglePin?: (exePath: string, exeName: string, displayName?: string) => Promise<boolean>;
  launchApp?: (path: string) => Promise<void>;
  hidePreview: () => Promise<void>;
}

const defaultBridge: PreviewBridge = {
  onShow: handler => listen<PreviewPayload>('preview://show', event => handler(event.payload)),
  capturePreview: (hwnd, pid) => invoke<PreviewEntry>('taskbar_capture_preview', { hwnd, pid }),
  activateWindow: (hwnd, pid) =>
    invoke<void>('taskbar_activate_window', { hwnd, pid, allowMinimize: false }),
  closeWindow: (hwnd, pid) => invoke<void>('taskbar_close_window', { hwnd, pid }),
  togglePin: (exePath, exeName, displayName) =>
    invoke<boolean>('taskbar_toggle_pin', { exePath, exeName, displayName }),
  launchApp: path => invoke<void>('taskbar_launch_app', { path }),
  hidePreview: () => invoke<void>('preview_hide'),
};

export function PreviewApp({ bridge = defaultBridge }: { bridge?: PreviewBridge }) {
  const [payload, setPayload] = useState<PreviewPayload | null>(null);
  const [thumbs, setThumbs] = useState<Record<number, PreviewEntry>>({});

  useEffect(() => {
    let active = true;
    let unlisten: UnlistenFn | null = null;

    bridge
      .onShow(next => {
        if (!active) return;
        setPayload(next);
        setThumbs({});
        if (next.mode !== 'context') {
          for (const win of next.group.windows.slice(0, 6)) {
            bridge
              .capturePreview(win.hwnd, win.pid)
              .then(entry => {
                if (!active) return;
                setThumbs(prev => ({ ...prev, [win.hwnd]: entry }));
              })
              .catch(() => {});
          }
        }
      })
      .then(fn => {
        if (!active) fn();
        else unlisten = fn;
      })
      .catch(() => {});

    return () => {
      active = false;
      unlisten?.();
    };
  }, [bridge]);

  if (!payload) return null;

  if (payload.mode === 'context') {
    const group = payload.group;
    const exeName = group.windows[0]?.exeName || group.key;
    return (
      <section
        data-testid="context-popout"
        onMouseLeave={() => void bridge.hidePreview()}
        style={{
          width: '100vw',
          height: '100vh',
          padding: 10,
          boxSizing: 'border-box',
          background: 'var(--m3-surface-container-high, #2b2930)',
          border: '1px solid var(--m3-outline-variant, #49454f)',
          borderRadius: 16,
          color: 'var(--m3-on-surface, #e6e0e9)',
          display: 'flex',
          flexDirection: 'column',
          gap: 6,
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: 8, padding: '2px 4px', fontWeight: 600, fontSize: 13 }}>
          {payload.iconUrl && <img src={payload.iconUrl} alt="" style={{ width: 16, height: 16 }} />}
          <span style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
            {group.displayName}
          </span>
        </div>

        <button
          type="button"
          className="state-layer"
          onClick={() => {
            void bridge.togglePin?.(group.exePath, exeName, group.displayName);
            void bridge.hidePreview();
          }}
          style={{ display: 'flex', alignItems: 'center', gap: 8, padding: '8px 10px', borderRadius: 10, fontSize: 13 }}
        >
          <Icon name={group.pinned ? 'keep_off' : 'keep'} />
          <span>{group.pinned ? 'Unpin from taskbar' : 'Pin to taskbar'}</span>
        </button>

        {group.exePath && (
          <button
            type="button"
            className="state-layer"
            onClick={() => {
              void bridge.launchApp?.(group.exePath);
              void bridge.hidePreview();
            }}
            style={{ display: 'flex', alignItems: 'center', gap: 8, padding: '8px 10px', borderRadius: 10, fontSize: 13 }}
          >
            <Icon name="open_in_new" />
            <span>Open new window</span>
          </button>
        )}

        {group.windowCount > 0 && (
          <button
            type="button"
            className="state-layer"
            onClick={() => {
              group.windows.forEach(w => void bridge.closeWindow(w.hwnd, w.pid));
              void bridge.hidePreview();
            }}
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 8,
              padding: '8px 10px',
              borderRadius: 10,
              fontSize: 13,
              color: 'var(--m3-error, #f2b8b5)',
            }}
          >
            <Icon name="close" />
            <span>{group.windowCount > 1 ? `Close all (${group.windowCount})` : 'Close window'}</span>
          </button>
        )}
      </section>
    );
  }

  if (payload.group.windows.length === 0) return null;

  return (
    <section
      data-testid="preview-popout"
      onMouseLeave={() => void bridge.hidePreview()}
      style={{
        width: '100vw',
        height: '100vh',
        padding: 10,
        boxSizing: 'border-box',
        background: 'rgba(22, 20, 28, 0.96)',
        border: '1px solid rgba(255, 255, 255, 0.14)',
        borderRadius: 18,
        color: '#e6e0e9',
        display: 'flex',
        flexDirection: 'column',
        gap: 8,
        overflowY: 'auto',
      }}
    >
      <header style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', fontSize: 12 }}>
        <strong style={{ color: 'var(--m3-primary, #d0bcff)' }}>{payload.group.displayName}</strong>
        <button
          type="button"
          title="Dismiss"
          onClick={() => void bridge.hidePreview()}
          style={{ width: 22, height: 22, borderRadius: 999, display: 'grid', placeItems: 'center' }}
        >
          <Icon name="close" />
        </button>
      </header>

      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(220px, 1fr))', gap: 8 }}>
        {payload.group.windows.map(win => {
          const entry = thumbs[win.hwnd];
          return (
            <article
              key={win.hwnd}
              data-testid={`preview-card-${win.hwnd}`}
              style={{
                background: win.active ? 'rgba(208, 188, 255, 0.14)' : 'rgba(255, 255, 255, 0.05)',
                border: win.active ? '1px solid rgba(208, 188, 255, 0.5)' : '1px solid rgba(255, 255, 255, 0.08)',
                borderRadius: 14,
                padding: 8,
                display: 'flex',
                flexDirection: 'column',
                gap: 6,
              }}
            >
              <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: 6 }}>
                <button
                  type="button"
                  onClick={() => {
                    void bridge.activateWindow(win.hwnd, win.pid);
                    void bridge.hidePreview();
                  }}
                  style={{
                    flex: 1,
                    textAlign: 'left',
                    fontSize: 12,
                    fontWeight: 500,
                    whiteSpace: 'nowrap',
                    overflow: 'hidden',
                    textOverflow: 'ellipsis',
                  }}
                  title={win.title}
                >
                  {win.title}
                </button>
                <button
                  type="button"
                  title="Close window"
                  aria-label={`Close ${win.title}`}
                  onClick={() => {
                    void bridge.closeWindow(win.hwnd, win.pid);
                    setPayload(curr =>
                      curr
                        ? {
                            ...curr,
                            group: {
                              ...curr.group,
                              windows: curr.group.windows.filter(w => w.hwnd !== win.hwnd),
                            },
                          }
                        : null,
                    );
                  }}
                  style={{
                    width: 22,
                    height: 22,
                    borderRadius: 999,
                    display: 'grid',
                    placeItems: 'center',
                    color: '#f2b8b5',
                  }}
                >
                  <Icon name="close" />
                </button>
              </div>

              <button
                type="button"
                aria-label={`Activate ${win.title}`}
                onClick={() => {
                  void bridge.activateWindow(win.hwnd, win.pid);
                  void bridge.hidePreview();
                }}
                style={{
                  width: '100%',
                  height: 118,
                  borderRadius: 10,
                  overflow: 'hidden',
                  background: '#141218',
                  display: 'grid',
                  placeItems: 'center',
                }}
              >
                {entry?.pngBase64 ? (
                  <img
                    src={entry.pngBase64}
                    alt={win.title}
                    style={{ width: '100%', height: '100%', objectFit: 'cover' }}
                  />
                ) : payload.iconUrl ? (
                  <img src={payload.iconUrl} alt="" style={{ width: 42, height: 42, objectFit: 'contain' }} />
                ) : (
                  <Icon name="web_asset" />
                )}
              </button>
            </article>
          );
        })}
      </div>
    </section>
  );
}
