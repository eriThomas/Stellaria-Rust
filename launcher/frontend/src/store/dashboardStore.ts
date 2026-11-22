import { create } from 'zustand';
import type {
  DashboardSettings,
  DashboardSnapshot,
  DashboardTab,
  FeatureFlags,
  LyricsResult,
  MediaSnapshot,
  MonitorInfo,
  SystemInfo,
  TelemetrySnapshot,
  VisibilityEvent,
  VisibilityState,
  WeatherSnapshot,
} from '../types/ipc';

const MAX_NETWORK_HISTORY = 40;

const defaultVisibility: VisibilityState = {
  open: false,
  monitorId: null,
  trigger: null,
  suppressed: false,
  activeTab: 'dash',
};

const defaultSettings: DashboardSettings = {
  schemaVersion: 1,
  language: 'auto',
  clockSeconds: false,
  lyricsEnabled: true,
  weatherEnabled: true,
  themeMode: 'dark',
  themeVariant: 'tonalspot',
  sourceColor: '#7e57c2',
  reducedMotion: false,
  gameMode: false,
  borderSize: 0,
};

const defaultFlags: FeatureFlags = {
  identity: 'experimental',
  dashboardPilot: true,
  zebarCoexistence: true,
};

export interface DashboardStoreState {
  revision: number;
  sessionGeneration: number;
  hydrated: boolean;
  visibility: VisibilityState;
  settings: DashboardSettings;
  monitors: MonitorInfo[];
  flags: FeatureFlags;
  sysinfo: SystemInfo | null;
  telemetry: TelemetrySnapshot | null;
  networkHistory: { down: number[]; up: number[] };
  media: MediaSnapshot | null;
  weather: WeatherSnapshot | null;
  weatherError: string | null;
  lyrics: LyricsResult | null;
  applySnapshot: (snapshot: DashboardSnapshot) => void;
  applyVisibilityEvent: (event: VisibilityEvent) => void;
  applySettingsUpdate: (revision: number, settings: DashboardSettings) => void;
  applyTelemetry: (telemetry: TelemetrySnapshot) => void;
  setSysInfo: (sysinfo: SystemInfo) => void;
  setMedia: (media: MediaSnapshot) => void;
  setWeather: (weather: WeatherSnapshot | null, error?: string | null) => void;
  setLyrics: (lyrics: LyricsResult | null) => void;
  setLocalTab: (tab: DashboardTab) => void;
  reset: () => void;
}

export const useDashboardStore = create<DashboardStoreState>(set => ({
  revision: 0,
  sessionGeneration: 0,
  hydrated: false,
  visibility: defaultVisibility,
  settings: defaultSettings,
  monitors: [],
  flags: defaultFlags,
  sysinfo: null,
  telemetry: null,
  networkHistory: { down: [], up: [] },
  media: null,
  weather: null,
  weatherError: null,
  lyrics: null,

  applySnapshot: snapshot =>
    set({
      revision: snapshot.revision,
      sessionGeneration: snapshot.sessionGeneration,
      hydrated: true,
      visibility: snapshot.visibility,
      settings: snapshot.settings,
      monitors: snapshot.monitors,
      flags: snapshot.flags,
    }),

  applyVisibilityEvent: event =>
    set(state => {
      if (event.revision < state.revision) return state;
      return {
        revision: event.revision,
        visibility: event.visibility,
      };
    }),

  applySettingsUpdate: (revision, settings) =>
    set(state => {
      if (revision < state.revision) return state;
      return {
        revision,
        settings,
      };
    }),

  applyTelemetry: telemetry =>
    set(state => {
      const down = [...state.networkHistory.down, telemetry.network.rxBytesPerSec].slice(-MAX_NETWORK_HISTORY);
      const up = [...state.networkHistory.up, telemetry.network.txBytesPerSec].slice(-MAX_NETWORK_HISTORY);
      return {
        telemetry,
        networkHistory: { down, up },
      };
    }),

  setSysInfo: sysinfo => set({ sysinfo }),
  setMedia: media => set({ media }),
  setWeather: (weather, error = null) => set({ weather, weatherError: error }),
  setLyrics: lyrics => set({ lyrics }),

  setLocalTab: tab =>
    set(state => ({
      visibility: { ...state.visibility, activeTab: tab },
    })),

  reset: () =>
    set({
      revision: 0,
      sessionGeneration: 0,
      hydrated: false,
      visibility: defaultVisibility,
      settings: defaultSettings,
      monitors: [],
      flags: defaultFlags,
      sysinfo: null,
      telemetry: null,
      networkHistory: { down: [], up: [] },
      media: null,
      weather: null,
      weatherError: null,
      lyrics: null,
    }),
}));
