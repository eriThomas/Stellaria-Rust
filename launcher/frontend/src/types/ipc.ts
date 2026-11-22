export type DashboardTab = 'dash' | 'media' | 'performance' | 'weather';
export type OpenTrigger = 'click' | 'hotkey' | 'tray' | 'hover';
export type ThemeMode = 'dark' | 'light';
export type ThemeVariant =
  | 'tonalspot'
  | 'vibrant'
  | 'expressive'
  | 'fidelity'
  | 'content'
  | 'fruitsalad'
  | 'rainbow'
  | 'neutral'
  | 'monochrome';

export interface PhysicalRect {
  left: number;
  top: number;
  right: number;
  bottom: number;
}

export interface MonitorInfo {
  id: string;
  name: string;
  bounds: PhysicalRect;
  workArea: PhysicalRect;
  scaleFactor: number;
  primary: boolean;
  fullscreen: boolean;
}

export interface DashboardSettings {
  schemaVersion: number;
  language: string;
  clockSeconds: boolean;
  lyricsEnabled: boolean;
  weatherEnabled: boolean;
  themeMode: ThemeMode;
  themeVariant: ThemeVariant;
  sourceColor: string;
  reducedMotion: boolean;
  gameMode: boolean;
  borderSize: number;
}

export type DashboardSettingsPatch = Partial<Omit<DashboardSettings, 'schemaVersion'>>;

export interface VisibilityState {
  open: boolean;
  monitorId: string | null;
  trigger: OpenTrigger | null;
  suppressed: boolean;
  activeTab: DashboardTab;
}

export interface FeatureFlags {
  identity: 'production' | 'experimental';
  dashboardPilot: boolean;
  zebarCoexistence: boolean;
}

export interface DashboardSnapshot {
  revision: number;
  sessionGeneration: number;
  visibility: VisibilityState;
  settings: DashboardSettings;
  monitors: MonitorInfo[];
  flags: FeatureFlags;
}

export interface VisibilityEvent {
  revision: number;
  visibility: VisibilityState;
}

export interface SettingsEvent {
  revision: number;
  settings: DashboardSettings;
}
