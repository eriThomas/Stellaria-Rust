import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type {
  DashboardSettings,
  DashboardSettingsPatch,
  DashboardSnapshot,
  DashboardTab,
  OpenTrigger,
  SettingsEvent,
  VisibilityEvent,
} from '../types/ipc';

export interface DashboardBridge {
  getSnapshot: () => Promise<DashboardSnapshot>;
  setVisibility: (open: boolean, monitorId: string | null, trigger: OpenTrigger) => Promise<VisibilityEvent>;
  setActiveTab: (tab: DashboardTab) => Promise<VisibilityEvent>;
  updateSettings: (patch: DashboardSettingsPatch) => Promise<DashboardSettings>;
  onVisibility: (handler: (event: VisibilityEvent) => void) => Promise<UnlistenFn>;
  onSettings: (handler: (event: SettingsEvent) => void) => Promise<UnlistenFn>;
}

export const tauriDashboardBridge: DashboardBridge = {
  getSnapshot: () => invoke<DashboardSnapshot>('dashboard_get_snapshot'),
  setVisibility: (open, monitorId, trigger) =>
    invoke<VisibilityEvent>('dashboard_set_visibility', {
      open,
      monitorId,
      trigger,
    }),
  setActiveTab: tab => invoke<VisibilityEvent>('dashboard_set_active_tab', { tab }),
  updateSettings: patch => invoke<DashboardSettings>('dashboard_update_settings', { patch }),
  onVisibility: handler =>
    listen<VisibilityEvent>('dashboard://visibility', event => handler(event.payload)),
  onSettings: handler =>
    listen<SettingsEvent>('dashboard://settings', event => handler(event.payload)),
};
