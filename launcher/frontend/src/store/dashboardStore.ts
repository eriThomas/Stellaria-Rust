import { create } from 'zustand';
import type {
  DashboardSettings,
  DashboardSnapshot,
  DashboardTab,
  FeatureFlags,
  MonitorInfo,
  VisibilityEvent,
  VisibilityState,
} from '../types/ipc';

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
  applySnapshot: (snapshot: DashboardSnapshot) => void;
  applyVisibilityEvent: (event: VisibilityEvent) => void;
  applySettingsUpdate: (revision: number, settings: DashboardSettings) => void;
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
      // Ignore stale or out-of-order events.
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
    }),
}));
