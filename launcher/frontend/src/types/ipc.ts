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

export type MediaAction = 'togglePlayPause' | 'next' | 'previous';

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

export interface DiskTelemetry {
  mountPoint: string;
  totalBytes: number;
  availableBytes: number;
}

export interface NetworkTelemetry {
  rxBytesPerSec: number;
  txBytesPerSec: number;
  totalRxBytes: number;
  totalTxBytes: number;
}

export interface GpuTelemetry {
  name: string;
  usagePercent: number | null;
  temperatureC: number | null;
}

export interface TelemetrySnapshot {
  revision: number;
  uptimeMs: number;
  cpuPercent: number;
  memoryUsedBytes: number;
  memoryTotalBytes: number;
  disks: DiskTelemetry[];
  network: NetworkTelemetry;
  gpu: GpuTelemetry | null;
}

export interface SystemInfo {
  os: string;
  cpu: string;
  gpu: string | null;
  user: string;
  pictureBase64: string | null;
}

export interface MediaSessionInfo {
  sessionId: string;
  title: string;
  artist: string;
  album: string;
  isPlaying: boolean;
  positionSecs: number;
  durationSecs: number;
}

export interface MediaSnapshot {
  current: MediaSessionInfo | null;
  sessions: MediaSessionInfo[];
  artworkBase64: string | null;
}

export interface CurrentWeather {
  temperature: number;
  feelsLike: number;
  humidity: number;
  wind: number;
  code: number;
  isDay: boolean;
  icon: string;
  conditionKey: string;
}

export interface DailyForecast {
  date: string;
  code: number;
  icon: string;
  max: number;
  min: number;
}

export interface WeatherSnapshot {
  fetchedAtMs: number;
  city: string;
  current: CurrentWeather;
  sunrise: string | null;
  sunset: string | null;
  daily: DailyForecast[];
}

export interface LyricLine {
  timeSecs: number;
  text: string;
}

export interface LyricsResult {
  key: string;
  lines: LyricLine[];
}
