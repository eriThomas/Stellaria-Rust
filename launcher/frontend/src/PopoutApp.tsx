import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useCallback, useEffect, useState } from 'react';
import { Icon } from './components/ExpressiveShape';
import { applyMaterialTheme } from './theme/materialYou';
import type { DashboardSnapshot } from './types/ipc';

export type PopoutKind = 'audio' | 'network' | 'keyboard' | 'notifications';

interface AudioSessionItem {
  id: string;
  name: string;
  system: boolean;
  volume: number;
  muted: boolean;
  icon?: string | null;
}

interface KeyboardLayoutItem {
  id: string;
  code: string;
  name: string;
  language: string;
}

interface KeyboardLayoutsPayload {
  current: string;
  items: KeyboardLayoutItem[];
}

interface NetworkInfoPayload {
  connected: boolean;
  adapter: string;
  ipv4: string;
  mac: string;
  linkSpeedMbps: number;
  rxBytesPerSec: number;
  txBytesPerSec: number;
}

interface NotificationItem {
  id: string;
  appId: string;
  title: string;
  body: string;
  time: number;
}

function formatRate(bytesPerSec: number): string {
  const units = ['B/s', 'KB/s', 'MB/s', 'GB/s'];
  let value = Math.max(0, bytesPerSec);
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit++;
  }
  return `${value.toFixed(value >= 100 || unit === 0 ? 0 : 1)} ${units[unit]}`;
}

function relativeTime(ms: number): string {
  if (!ms) return '';
  const mins = Math.max(0, Math.round((Date.now() - ms) / 60000));
  if (mins < 1) return 'Just now';
  if (mins < 60) return `${mins} min ago`;
  const hrs = Math.round(mins / 60);
  if (hrs < 24) return `${hrs} h ago`;
  return new Date(ms).toLocaleDateString();
}

function fallbackAppName(appId: string): string {
  const base = appId.split('!')[0].split('_')[0];
  const last = base.split('.').pop() ?? appId;
  return last.replace(/[-_]+/g, ' ');
}

function MaterialSlider({
  value,
  muted,
  onChange,
}: {
  value: number;
  muted: boolean;
  onChange: (next: number) => void;
}) {
  const pct = muted ? 0 : Math.min(100, Math.max(0, value));
  return (
    <div
      className="slider"
      onPointerDown={e => {
        const rect = e.currentTarget.getBoundingClientRect();
        const next = Math.round(Math.min(1, Math.max(0, (e.clientX - rect.left) / rect.width)) * 100);
        onChange(next);
      }}
      onWheel={e => {
        e.preventDefault();
        const delta = e.deltaY < 0 ? 4 : -4;
        onChange(Math.min(100, Math.max(0, value + delta)));
      }}
    >
      <div className="slider-active" style={{ width: `calc(${pct}% - 4px)` }} />
      <div className="slider-inactive" style={{ left: `calc(${pct}% + 4px)` }} />
      <div className="slider-handle" style={{ left: `${pct}%` }} />
      <span className="icon slider-icon" style={{ opacity: pct > 18 ? 1 : 0 }}>
        {muted || pct === 0 ? 'volume_off' : 'volume_up'}
      </span>
    </div>
  );
}

export function PopoutApp() {
  const [kind, setKind] = useState<PopoutKind>('audio');
  const [open, setOpen] = useState(true);
  const [sessions, setSessions] = useState<AudioSessionItem[]>([]);
  const [sessionsLoading, setSessionsLoading] = useState(false);
  const [layouts, setLayouts] = useState<KeyboardLayoutsPayload>({ current: '', items: [] });
  const [network, setNetwork] = useState<NetworkInfoPayload | null>(null);
  const [notifications, setNotifications] = useState<NotificationItem[]>([]);
  const [dismissedIds, setDismissedIds] = useState<string[]>(() => {
    try {
      return JSON.parse(localStorage.getItem('stellaria:dismissed-notifs') || '[]');
    } catch {
      return [];
    }
  });

  const hide = useCallback(() => {
    setOpen(false);
    void invoke('popout_hide');
  }, []);

  const loadAudio = useCallback(() => {
    setSessionsLoading(true);
    invoke<AudioSessionItem[]>('popout_get_audio_sessions')
      .then(list => setSessions(list))
      .catch(() => setSessions([]))
      .finally(() => setSessionsLoading(false));
  }, []);

  const loadKeyboard = useCallback(() => {
    invoke<KeyboardLayoutsPayload>('popout_get_keyboard_layouts')
      .then(data => setLayouts(data))
      .catch(() => {});
  }, []);

  const loadNetwork = useCallback(() => {
    invoke<NetworkInfoPayload>('popout_get_network_info')
      .then(data => setNetwork(data))
      .catch(() => {});
  }, []);

  const loadNotifications = useCallback(() => {
    invoke<NotificationItem[]>('popout_get_notifications')
      .then(list => setNotifications(list))
      .catch(() => setNotifications([]));
  }, []);

  useEffect(() => {
    invoke<DashboardSnapshot>('dashboard_get_snapshot')
      .then(s => applyMaterialTheme(s.settings))
      .catch(() => {});

    document.body.classList.add('open');

    const unlistenShow = listen<{ kind: PopoutKind }>('popout://show', ev => {
      const nextKind = ev.payload.kind;
      setKind(nextKind);
      setOpen(true);
      document.body.classList.remove('closing');
      document.body.classList.add('open');
      if (nextKind === 'audio') loadAudio();
      if (nextKind === 'keyboard') loadKeyboard();
      if (nextKind === 'network') loadNetwork();
      if (nextKind === 'notifications') loadNotifications();
    });

    const unlistenHide = listen<void>('popout://hide', () => {
      setOpen(false);
      document.body.classList.remove('open');
      document.body.classList.add('closing');
    });

    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') hide();
    };
    const onBlur = () => hide();

    window.addEventListener('keydown', onKeyDown);
    window.addEventListener('blur', onBlur);
    return () => {
      window.removeEventListener('keydown', onKeyDown);
      window.removeEventListener('blur', onBlur);
      void unlistenShow.then(fn => fn());
      void unlistenHide.then(fn => fn());
    };
  }, [hide, loadAudio, loadKeyboard, loadNetwork, loadNotifications]);

  // Poll live network rates only while the network popout is open
  useEffect(() => {
    if (!open || kind !== 'network') return;
    loadNetwork();
    const id = window.setInterval(loadNetwork, 1000);
    return () => window.clearInterval(id);
  }, [open, kind, loadNetwork]);

  const dismissNotification = (id: string) => {
    const next = [...new Set([...dismissedIds, id])].slice(-200);
    setDismissedIds(next);
    try {
      localStorage.setItem('stellaria:dismissed-notifs', JSON.stringify(next));
    } catch {
      // ignore storage errors
    }
  };

  const visibleNotifications = notifications.filter(n => !dismissedIds.includes(n.id));

  return (
    <div
      className="drawer from-left"
      style={{ width: '100%', height: '100%', display: 'flex', alignItems: 'flex-end' }}
      onMouseDown={e => {
        if (e.target === e.currentTarget) hide();
      }}
    >
      <div className="drawer-anchor">
        <div className="flare start" />
        <section className="drawer-panel popout" id="panel">
          <div className="drawer-content">
            <div className="popout-viewport">
              <div className="popout-page">
                {kind === 'audio' && (
                  <div className="view" style={{ width: '340px' }}>
                    <div className="view-header">
                      <Icon name="volume_up" />
                      <span className="view-title">Volume</span>
                    </div>
                    <div className="button-line">
                      <button
                        type="button"
                        className="tonal-button"
                        onClick={() => void invoke('shell_volume_action', { action: 'mute' })}
                      >
                        <Icon name="volume_off" />
                        <span>Mute</span>
                      </button>
                      <button
                        type="button"
                        className="tonal-button"
                        onClick={() => void invoke('shell_volume_action', { action: 'down' })}
                      >
                        <Icon name="volume_down" />
                        <span>-</span>
                      </button>
                      <button
                        type="button"
                        className="tonal-button"
                        onClick={() => void invoke('shell_volume_action', { action: 'up' })}
                      >
                        <Icon name="volume_up" />
                        <span>+</span>
                      </button>
                    </div>
                    <span className="section-label">Applications</span>
                    <div className="app-volumes" style={{ maxHeight: '240px', overflowY: 'auto' }}>
                      {sessionsLoading && sessions.length === 0 ? (
                        <span className="empty">Loading…</span>
                      ) : sessions.length === 0 ? (
                        <span className="empty">No active audio apps</span>
                      ) : (
                        sessions.map(session => {
                          const label = session.system ? 'System sounds' : session.name;
                          return (
                            <div key={session.id} className="app-volume" title={label}>
                              {session.icon ? (
                                <img
                                  className="app-icon"
                                  src={`data:image/png;base64,${session.icon}`}
                                  alt=""
                                />
                              ) : (
                                <Icon name={session.system ? 'notifications' : 'apps'} className="app-icon" />
                              )}
                              <div className="app-volume-body">
                                <span className="app-name">{label}</span>
                                <MaterialSlider
                                  value={session.volume}
                                  muted={session.muted}
                                  onChange={next => {
                                    setSessions(prev =>
                                      prev.map(s => (s.id === session.id ? { ...s, volume: next, muted: false } : s)),
                                    );
                                    void invoke('popout_set_audio_session_volume', { id: session.id, volume: next });
                                  }}
                                />
                              </div>
                              <button
                                type="button"
                                className={`icon-button${session.muted ? ' active' : ''}`}
                                title={session.muted ? 'Unmute' : 'Mute'}
                                onClick={() => {
                                  const nextMuted = !session.muted;
                                  setSessions(prev =>
                                    prev.map(s => (s.id === session.id ? { ...s, muted: nextMuted } : s)),
                                  );
                                  void invoke('popout_set_audio_session_mute', { id: session.id, muted: nextMuted });
                                }}
                              >
                                <Icon name={session.muted || session.volume === 0 ? 'volume_off' : 'volume_up'} />
                              </button>
                            </div>
                          );
                        })
                      )}
                    </div>
                    <button
                      type="button"
                      className="tonal-button"
                      onClick={() => {
                        void invoke('shell_open_settings', { page: 'sound' });
                        hide();
                      }}
                    >
                      <Icon name="settings" />
                      <span>Sound settings</span>
                    </button>
                  </div>
                )}

                {kind === 'network' && (
                  <div className="view" style={{ width: '320px' }}>
                    <div className="view-header">
                      <Icon name={network?.connected ? 'lan' : 'signal_wifi_off'} />
                      <span className="view-title">{network?.connected ? network.adapter : 'Offline'}</span>
                    </div>
                    <div className="info-grid">
                      <span>Status</span>
                      <span>{network?.connected ? 'Connected' : 'No connection'}</span>
                      <span>IPv4</span>
                      <span>{network?.ipv4 || '—'}</span>
                      <span>Speed</span>
                      <span>
                        {network && network.linkSpeedMbps > 0
                          ? network.linkSpeedMbps >= 1000
                            ? `${(network.linkSpeedMbps / 1000).toFixed(1).replace('.0', '')} Gb/s`
                            : `${network.linkSpeedMbps} Mb/s`
                          : '—'}
                      </span>
                      <span>MAC</span>
                      <span>{network?.mac || '—'}</span>
                    </div>
                    <div className="traffic">
                      <div className="down">
                        <Icon name="download" filled />
                        <span className="traffic-value">
                          <strong>{formatRate(network?.rxBytesPerSec ?? 0)}</strong>
                          <span>Download</span>
                        </span>
                      </div>
                      <div className="up">
                        <Icon name="upload" filled />
                        <span className="traffic-value">
                          <strong>{formatRate(network?.txBytesPerSec ?? 0)}</strong>
                          <span>Upload</span>
                        </span>
                      </div>
                    </div>
                    <button
                      type="button"
                      className="tonal-button"
                      onClick={() => {
                        void invoke('shell_open_settings', { page: 'network-status' });
                        hide();
                      }}
                    >
                      <Icon name="settings" />
                      <span>Network settings</span>
                    </button>
                  </div>
                )}

                {kind === 'keyboard' && (
                  <div className="view" style={{ width: '300px' }}>
                    <div className="view-header">
                      <Icon name="keyboard" />
                      <span className="view-title">Keyboard layout</span>
                    </div>
                    <div className="options">
                      {layouts.items.length === 0 ? (
                        <span className="hint">Loading layouts…</span>
                      ) : (
                        layouts.items.map(item => {
                          const selected = item.id.toLowerCase() === layouts.current.toLowerCase();
                          return (
                            <button
                              key={item.id}
                              type="button"
                              className={`option${selected ? ' selected' : ''}`}
                              onClick={() => {
                                setLayouts(prev => ({ ...prev, current: item.id }));
                                void invoke('popout_set_keyboard_layout', { id: item.id });
                              }}
                            >
                              <Icon name={selected ? 'radio_button_checked' : 'keyboard'} filled={selected} />
                              <span className="option-text">
                                <span>{item.name}</span>
                                <span>{item.language}</span>
                              </span>
                            </button>
                          );
                        })
                      )}
                    </div>
                    <span className="hint">Win + Space also switches between layouts.</span>
                    <button
                      type="button"
                      className="tonal-button"
                      onClick={() => {
                        void invoke('shell_open_settings', { page: 'regionlanguage' });
                        hide();
                      }}
                    >
                      <Icon name="settings" />
                      <span>Language &amp; keyboard</span>
                    </button>
                  </div>
                )}

                {kind === 'notifications' && (
                  <div className="view notifications-view" style={{ width: '340px' }}>
                    <div className="view-header">
                      <Icon name="notifications" />
                      <span className="view-title">Notifications</span>
                      <span className="view-value">
                        {visibleNotifications.length ? visibleNotifications.length : 'All quiet'}
                      </span>
                    </div>
                    <div className="notification-list" style={{ maxHeight: '280px', overflowY: 'auto' }}>
                      {visibleNotifications.length === 0 ? (
                        <div className="notification-empty">
                          <Icon name="notifications_paused" className="tonal-icon" />
                          <span>No notifications right now</span>
                        </div>
                      ) : (
                        visibleNotifications.slice(0, 15).map(entry => (
                          <article key={entry.id} className="notification">
                            <div className="notification-text">
                              <span className="notification-title">
                                {entry.title || fallbackAppName(entry.appId)}
                              </span>
                              {entry.body && <span className="notification-body">{entry.body}</span>}
                              <span className="notification-time">{relativeTime(entry.time)}</span>
                            </div>
                            <button
                              type="button"
                              className="icon-button notification-dismiss"
                              title="Dismiss"
                              onClick={() => dismissNotification(entry.id)}
                            >
                              <Icon name="close" />
                            </button>
                          </article>
                        ))
                      )}
                    </div>
                    <div className="button-line">
                      {visibleNotifications.length > 0 && (
                        <button
                          type="button"
                          className="tonal-button"
                          onClick={() => {
                            const next = [
                              ...new Set([...dismissedIds, ...visibleNotifications.map(n => n.id)]),
                            ].slice(-200);
                            setDismissedIds(next);
                            try {
                              localStorage.setItem('stellaria:dismissed-notifs', JSON.stringify(next));
                            } catch {
                              // ignore
                            }
                          }}
                        >
                          <Icon name="clear_all" />
                          <span>Clear all</span>
                        </button>
                      )}
                      <button
                        type="button"
                        className="tonal-button"
                        onClick={() => {
                          void invoke('shell_open_settings', { page: 'notifications' });
                          hide();
                        }}
                      >
                        <Icon name="settings" />
                        <span>Settings</span>
                      </button>
                    </div>
                  </div>
                )}
              </div>
            </div>
          </div>
        </section>
        <div className="flare end" />
      </div>
    </div>
  );
}
