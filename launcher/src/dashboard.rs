use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use stellaria_core::flags::{log_path, settings_path, FeatureFlags};
use stellaria_core::logging::StructuredLogger;
use stellaria_core::monitors::{compute_top_dashboard_placement, MonitorInfo, PhysicalRect};
use stellaria_core::settings::{DashboardSettings, DashboardSettingsPatch};
use stellaria_core::state::{DashboardCore, DashboardSnapshot, DashboardTab, OpenTrigger, VisibilityEvent};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, Runtime, State, WebviewUrl, WebviewWindowBuilder};
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetWindowLongPtrW, SetWindowLongPtrW, SetWindowPos, GWL_EXSTYLE, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER,
    WS_EX_APPWINDOW, WS_EX_TOOLWINDOW,
};

pub const DASHBOARD_LABEL: &str = "dashboard";
const DEFAULT_LOGICAL_WIDTH: f64 = 960.0;
const DEFAULT_LOGICAL_HEIGHT: f64 = 560.0;

pub struct DashboardHostState {
    pub core: Mutex<DashboardCore>,
    pub logger: StructuredLogger,
    settings_file: PathBuf,
    started_at: Instant,
}

impl DashboardHostState {
    pub fn initialize(local_app_data: &std::path::Path) -> Self {
        let flags = FeatureFlags::from_env();
        let session_generation = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(1);
        let settings_file = settings_path(local_app_data, flags.identity);
        let log_file = log_path(local_app_data, flags.identity);
        let settings = DashboardSettings::load_or_default(&settings_file);
        let logger = StructuredLogger::new(Some(log_file), session_generation);
        let core = DashboardCore::new(session_generation, settings, flags);
        Self {
            core: Mutex::new(core),
            logger,
            settings_file,
            started_at: Instant::now(),
        }
    }

    pub fn elapsed_ms(&self) -> u64 {
        self.started_at.elapsed().as_millis() as u64
    }
}

/// Applies WS_EX_TOOLWINDOW and removes WS_EX_APPWINDOW so internal windows never appear in Windows Alt+Tab.
pub fn exclude_from_alt_tab(hwnd: HWND) {
    unsafe {
        let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let updated = (ex_style | WS_EX_TOOLWINDOW as isize) & !(WS_EX_APPWINDOW as isize);
        if updated != ex_style {
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, updated);
            SetWindowPos(
                hwnd,
                std::ptr::null_mut(),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
            );
        }
    }
}

fn collect_tauri_monitors<R: Runtime>(app: &AppHandle<R>) -> Vec<MonitorInfo> {
    let primary_name = app
        .primary_monitor()
        .ok()
        .flatten()
        .and_then(|m| m.name().cloned());
    let Ok(monitors) = app.available_monitors() else {
        return Vec::new();
    };
    monitors
        .into_iter()
        .enumerate()
        .map(|(index, mon)| {
            let name = mon
                .name()
                .cloned()
                .unwrap_or_else(|| format!("MONITOR_{index}"));
            let pos = mon.position();
            let size = mon.size();
            let bounds = PhysicalRect {
                left: pos.x,
                top: pos.y,
                right: pos.x + size.width as i32,
                bottom: pos.y + size.height as i32,
            };
            let work = mon.work_area();
            let work_area = PhysicalRect {
                left: work.position.x,
                top: work.position.y,
                right: work.position.x + work.size.width as i32,
                bottom: work.position.y + work.size.height as i32,
            };
            let primary = primary_name.as_deref() == Some(name.as_str());
            MonitorInfo {
                id: name.clone(),
                name,
                bounds,
                work_area,
                scale_factor: mon.scale_factor(),
                primary,
                fullscreen: false,
            }
        })
        .collect()
}

pub fn ensure_dashboard_window<R: Runtime>(app: &AppHandle<R>) -> Result<tauri::WebviewWindow<R>, String> {
    if let Some(existing) = app.get_webview_window(DASHBOARD_LABEL) {
        return Ok(existing);
    }

    let window = WebviewWindowBuilder::new(app, DASHBOARD_LABEL, WebviewUrl::App("dashboard.html".into()))
        .title("Stellaria Dashboard")
        .decorations(false)
        .transparent(true)
        .resizable(false)
        .skip_taskbar(true)
        .always_on_top(true)
        .visible(false)
        .focused(false)
        .inner_size(DEFAULT_LOGICAL_WIDTH, DEFAULT_LOGICAL_HEIGHT)
        .build()
        .map_err(|e| e.to_string())?;

    if let Ok(handle) = window.hwnd() {
        exclude_from_alt_tab(handle.0 as HWND);
    }

    Ok(window)
}

pub fn toggle_from_tray<R: Runtime>(app: &AppHandle<R>) {
    let state = app.state::<DashboardHostState>();
    let currently_open = state
        .core
        .lock()
        .map(|core| core.visibility().open)
        .unwrap_or(false);
    let _ = apply_visibility(app, &state, !currently_open, None, OpenTrigger::Tray);
}

fn apply_visibility<R: Runtime>(
    app: &AppHandle<R>,
    state: &DashboardHostState,
    open: bool,
    monitor_id: Option<String>,
    trigger: OpenTrigger,
) -> Result<VisibilityEvent, String> {
    let monitors = collect_tauri_monitors(app);
    let (event, target_monitor) = {
        let mut core = state.core.lock().map_err(|_| "busy".to_string())?;
        if !core.flags().dashboard_pilot {
            return Err("dashboard-pilot-disabled".to_string());
        }
        core.update_monitors(monitors.clone());
        let event = core.set_visibility(open, monitor_id, trigger)?;
        let target = event
            .visibility
            .monitor_id
            .as_ref()
            .and_then(|id| monitors.iter().find(|m| &m.id == id))
            .cloned();
        (event, target)
    };

    if event.visibility.open {
        let window = ensure_dashboard_window(app)?;
        if let Ok(handle) = window.hwnd() {
            exclude_from_alt_tab(handle.0 as HWND);
        }
        if let Some(mon) = target_monitor {
            let placement = compute_top_dashboard_placement(&mon, DEFAULT_LOGICAL_WIDTH, DEFAULT_LOGICAL_HEIGHT);
            let _ = window.set_position(PhysicalPosition::new(placement.x, placement.y));
            let _ = window.set_size(PhysicalSize::new(placement.width, placement.height));
        }
        let _ = crate::foreground::present(&window);
        state.logger.info("dashboard", "opened");
    } else if let Some(window) = app.get_webview_window(DASHBOARD_LABEL) {
        let _ = window.hide();
        state.logger.info("dashboard", "closed");
    }

    let _ = app.emit("dashboard://visibility", &event);
    Ok(event)
}

#[tauri::command]
pub fn dashboard_get_snapshot(app: AppHandle, state: State<'_, DashboardHostState>) -> Result<DashboardSnapshot, String> {
    let monitors = collect_tauri_monitors(&app);
    let mut core = state.core.lock().map_err(|_| "busy".to_string())?;
    core.update_monitors(monitors);
    Ok(core.snapshot())
}

#[tauri::command]
pub fn dashboard_set_visibility(
    app: AppHandle,
    state: State<'_, DashboardHostState>,
    open: bool,
    monitor_id: Option<String>,
    trigger: OpenTrigger,
) -> Result<VisibilityEvent, String> {
    apply_visibility(&app, &state, open, monitor_id, trigger)
}

#[tauri::command]
pub fn dashboard_set_active_tab(
    app: AppHandle,
    state: State<'_, DashboardHostState>,
    tab: DashboardTab,
) -> Result<VisibilityEvent, String> {
    let event = {
        let mut core = state.core.lock().map_err(|_| "busy".to_string())?;
        core.set_active_tab(tab)
    };
    let _ = app.emit("dashboard://visibility", &event);
    Ok(event)
}

#[tauri::command]
pub fn dashboard_update_settings(
    app: AppHandle,
    state: State<'_, DashboardHostState>,
    patch: DashboardSettingsPatch,
) -> Result<DashboardSettings, String> {
    let (revision, updated) = {
        let mut core = state.core.lock().map_err(|_| "busy".to_string())?;
        core.update_settings(patch)?
    };
    updated.save_atomic(&state.settings_file)?;
    let _ = app.emit("dashboard://settings", serde_json::json!({ "revision": revision, "settings": &updated }));
    state.logger.info("settings", "updated");
    Ok(updated)
}
