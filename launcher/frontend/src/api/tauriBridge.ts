import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type {
  DashboardSettings,
  DashboardSettingsPatch,
  DashboardSnapshot,
  DashboardTab,
  LyricsResult,
  MediaAction,
  MediaSnapshot,
  OpenTrigger,
  SettingsEvent,
  SystemInfo,
  TelemetrySnapshot,
  VisibilityEvent,
  WeatherSnapshot,
} from '../types/ipc';

export interface DashboardBridge {
  getSnapshot: () => Promise<DashboardSnapshot>;
  setVisibility: (open: boolean, monitorId: string | null, trigger: OpenTrigger) => Promise<VisibilityEvent>;
  setActiveTab: (tab: DashboardTab) => Promise<VisibilityEvent>;
  updateSettings: (patch: DashboardSettingsPatch) => Promise<DashboardSettings>;
  getSysInfo: () => Promise<SystemInfo>;
  getTelemetry: () => Promise<TelemetrySnapshot>;
  getMedia: () => Promise<MediaSnapshot>;
  sendMediaAction: (action: MediaAction) => Promise<void>;
  refreshWeather: (force?: boolean) => Promise<WeatherSnapshot | null>;
  getLyrics: (artist: string, title: string, durationSecs?: number) => Promise<LyricsResult>;
  onVisibility: (handler: (event: VisibilityEvent) => void) => Promise<UnlistenFn>;
  onSettings: (handler: (event: SettingsEvent) => void) => Promise<UnlistenFn>;
  onTelemetry: (handler: (snapshot: TelemetrySnapshot) => void) => Promise<UnlistenFn>;
}

export const tauriDashboardBridge: DashboardBridge = {
  getSnapshot: () => invoke<DashboardSnapshot>('dashboard_get_snapshot'),
  setVisibility: (open, monitorId, trigger) =>
    invoke<VisibilityEvent>('dashboard_set_visibility', { open, monitorId, trigger }),
  setActiveTab: tab => invoke<VisibilityEvent>('dashboard_set_active_tab', { tab }),
  updateSettings: patch => invoke<DashboardSettings>('dashboard_update_settings', { patch }),
  getSysInfo: () => invoke<SystemInfo>('dashboard_get_sysinfo'),
  getTelemetry: () => invoke<TelemetrySnapshot>('dashboard_get_telemetry'),
  getMedia: () => invoke<MediaSnapshot>('dashboard_get_media'),
  sendMediaAction: action => invoke<void>('dashboard_media_action', { action }),
  refreshWeather: (force = false) =>
    invoke<WeatherSnapshot | null>('dashboard_refresh_weather', { force }),
  getLyrics: (artist, title, durationSecs) =>
    invoke<LyricsResult>('dashboard_get_lyrics', { artist, title, durationSecs }),
  onVisibility: handler =>
    listen<VisibilityEvent>('dashboard://visibility', event => handler(event.payload)),
  onSettings: handler =>
    listen<SettingsEvent>('dashboard://settings', event => handler(event.payload)),
  onTelemetry: handler =>
    listen<TelemetrySnapshot>('dashboard://telemetry', event => handler(event.payload)),
};
