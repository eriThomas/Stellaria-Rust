use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use stellaria_core::flags::{log_path, settings_path, FeatureFlags};
use stellaria_core::logging::StructuredLogger;
use stellaria_core::lyrics::LyricsResult;
use stellaria_core::monitors::{compute_top_dashboard_placement, monitor_at_point, MonitorInfo, PhysicalRect};
use stellaria_core::settings::{DashboardSettings, DashboardSettingsPatch};
use stellaria_core::state::{DashboardCore, DashboardSnapshot, DashboardTab, OpenTrigger, VisibilityEvent};
use stellaria_core::telemetry::{GpuTelemetry, MediaAction, MediaSnapshot, SystemInfo, TelemetrySnapshot};
use stellaria_core::weather::WeatherSnapshot;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, Runtime, State, WebviewUrl, WebviewWindowBuilder};
use windows_sys::Win32::Foundation::{HWND, POINT};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetCursorPos, GetWindowLongPtrW, SetWindowLongPtrW, SetWindowPos, ShowWindow, GWL_EXSTYLE, HWND_TOPMOST, SWP_FRAMECHANGED,
    SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SWP_SHOWWINDOW, SW_SHOWNOACTIVATE, WS_EX_APPWINDOW, WS_EX_TOOLWINDOW,
};

use crate::helper_bridge;
use crate::telemetry::SystemMetricsSampler;
use crate::web_services::{self, WebServicesState};

pub const DASHBOARD_LABEL: &str = "dashboard";
const DEFAULT_LOGICAL_WIDTH: f64 = 1040.0;
const DEFAULT_LOGICAL_HEIGHT: f64 = 560.0;
const HOVER_TRIGGER_HALF_WIDTH: f64 = 450.0;
const HOVER_CLOSE_GRACE_MS: u64 = 180;
const HOVER_REOPEN_COOLDOWN_MS: u64 = 350;
const CLOSE_ANIMATION_MS: u64 = 340;

pub struct DashboardHostState {
    pub core: Mutex<DashboardCore>,
    pub sampler: Mutex<SystemMetricsSampler>,
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
            sampler: Mutex::new(SystemMetricsSampler::default()),
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
        .shadow(false)
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
            if let Ok(handle) = window.hwnd() {
                unsafe {
                    SetWindowPos(
                        handle.0 as HWND,
                        HWND_TOPMOST,
                        placement.x,
                        placement.y,
                        placement.width as i32,
                        placement.height as i32,
                        SWP_NOACTIVATE | SWP_SHOWWINDOW,
                    );
                }
            }
        }
        if trigger == OpenTrigger::Hover {
            if let Ok(handle) = window.hwnd() {
                unsafe { ShowWindow(handle.0 as HWND, SW_SHOWNOACTIVATE) };
            }
            let _ = window.show();
        } else {
            let _ = crate::foreground::present(&window);
        }
        state.logger.info("dashboard", "opened");
        let _ = app.emit("dashboard://visibility", &event);
    } else {
        let _ = app.emit("dashboard://visibility", &event);
        state.logger.info("dashboard", "closed");
        // Allow the CSS slide-up drawer transition to finish before hiding the native HWND.
        let app_clone = app.clone();
        let expected_rev = event.revision;
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(CLOSE_ANIMATION_MS));
            let still_closed = app_clone
                .state::<DashboardHostState>()
                .core
                .lock()
                .map(|c| !c.visibility().open && c.snapshot().revision >= expected_rev)
                .unwrap_or(true);
            if still_closed {
                if let Some(window) = app_clone.get_webview_window(DASHBOARD_LABEL) {
                    let _ = window.hide();
                }
            }
        });
    }

    Ok(event)
}

fn cursor_pos() -> Option<(i32, i32)> {
    let mut pt = POINT { x: 0, y: 0 };
    if unsafe { GetCursorPos(&mut pt) } != 0 {
        Some((pt.x, pt.y))
    } else {
        None
    }
}

/// Native Win32 top-edge hover watcher: opens the dashboard when the cursor touches the top-center
/// of any non-fullscreen monitor, and smoothly closes it when the cursor leaves the panel.
pub fn start_hover_edge_watcher(app: AppHandle) {
    std::thread::spawn(move || {
        let mut last_closed_at = Instant::now() - Duration::from_secs(5);
        let mut outside_since: Option<Instant> = None;

        loop {
            let (is_open, trigger, active_monitor_id) = match app.state::<DashboardHostState>().core.lock() {
                Ok(core) => {
                    let vis = core.visibility();
                    (vis.open, vis.trigger, vis.monitor_id.clone())
                }
                Err(_) => (false, None, None),
            };

            std::thread::sleep(Duration::from_millis(if is_open { 50 } else { 80 }));

            let Some((cx, cy)) = cursor_pos() else {
                continue;
            };

            if !is_open {
                outside_since = None;
                if last_closed_at.elapsed().as_millis() < HOVER_REOPEN_COOLDOWN_MS as u128 {
                    continue;
                }
                let monitors = collect_tauri_monitors(&app);
                let Some(mon) = monitor_at_point(&monitors, cx, cy) else {
                    continue;
                };
                if mon.fullscreen {
                    continue;
                }
                let scale = mon.scale_factor.max(1.0);
                let center_x = mon.bounds.left + mon.bounds.width() / 2;
                let half_zone = (HOVER_TRIGGER_HALF_WIDTH * scale).round() as i32;
                let at_top_edge = cy <= mon.bounds.top + 6 && cy >= mon.bounds.top - 2;
                let in_center_zone = (cx - center_x).abs() <= half_zone;

                if at_top_edge && in_center_zone {
                    let own_hwnd = app
                        .get_webview_window(DASHBOARD_LABEL)
                        .and_then(|w| w.hwnd().ok())
                        .map(|h| h.0 as HWND)
                        .unwrap_or(std::ptr::null_mut());
                    if crate::foreground::is_true_fullscreen(own_hwnd) {
                        continue;
                    }
                    let state = app.state::<DashboardHostState>();
                    let _ = apply_visibility(&app, &state, true, Some(mon.id.clone()), OpenTrigger::Hover);
                }
            } else if trigger == Some(OpenTrigger::Hover) {
                let monitors = collect_tauri_monitors(&app);
                let mon = active_monitor_id
                    .as_deref()
                    .and_then(|id| monitors.iter().find(|m| m.id == id))
                    .or_else(|| monitor_at_point(&monitors, cx, cy));
                let Some(mon) = mon else {
                    continue;
                };
                let placement = compute_top_dashboard_placement(mon, DEFAULT_LOGICAL_WIDTH, DEFAULT_LOGICAL_HEIGHT);
                let margin = (24.0 * mon.scale_factor.max(1.0)).round() as i32;
                let inside = cx >= placement.x - margin
                    && cx <= placement.x + placement.width as i32 + margin
                    && cy >= placement.y - 4
                    && cy <= placement.y + placement.height as i32;

                if inside {
                    outside_since = None;
                } else {
                    let since = outside_since.get_or_insert_with(Instant::now);
                    if since.elapsed().as_millis() >= HOVER_CLOSE_GRACE_MS as u128 {
                        let state = app.state::<DashboardHostState>();
                        let _ = apply_visibility(&app, &state, false, Some(mon.id.clone()), OpenTrigger::Hover);
                        last_closed_at = Instant::now();
                        outside_since = None;
                    }
                }
            }
        }
    });
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

#[tauri::command]
pub async fn dashboard_get_sysinfo() -> Result<SystemInfo, String> {
    helper_bridge::fetch_system_info().await
}

#[tauri::command]
pub async fn dashboard_get_telemetry(state: State<'_, DashboardHostState>) -> Result<TelemetrySnapshot, String> {
    let want_gpu = state
        .core
        .lock()
        .map(|core| core.should_poll_gpu())
        .unwrap_or(false);
    let gpu: Option<GpuTelemetry> = if want_gpu {
        helper_bridge::fetch_gpu_telemetry().await.ok()
    } else {
        None
    };
    let mut sampler = state.sampler.lock().map_err(|_| "busy".to_string())?;
    Ok(sampler.sample(gpu))
}

#[tauri::command]
pub async fn dashboard_get_media() -> Result<MediaSnapshot, String> {
    helper_bridge::fetch_media_snapshot().await
}

#[tauri::command]
pub async fn dashboard_media_action(action: MediaAction) -> Result<(), String> {
    helper_bridge::send_media_action(action).await
}

#[tauri::command]
pub async fn dashboard_refresh_weather(
    state: State<'_, DashboardHostState>,
    web: State<'_, WebServicesState>,
    force: bool,
) -> Result<Option<WeatherSnapshot>, String> {
    let enabled = state
        .core
        .lock()
        .map(|core| core.settings().weather_enabled)
        .unwrap_or(true);
    web_services::fetch_weather(&web, enabled, force).await
}

#[tauri::command]
pub async fn dashboard_get_lyrics(
    state: State<'_, DashboardHostState>,
    web: State<'_, WebServicesState>,
    artist: String,
    title: String,
    duration_secs: Option<f64>,
) -> Result<LyricsResult, String> {
    let enabled = state
        .core
        .lock()
        .map(|core| core.settings().lyrics_enabled)
        .unwrap_or(true);
    if !enabled {
        return Ok(LyricsResult {
            key: String::new(),
            lines: Vec::new(),
        });
    }
    web_services::search_lyrics(&web, artist, title, duration_secs).await
}

/// Visibility-gated background telemetry loop. Performs ZERO native sampling while the dashboard is hidden.
pub fn start_telemetry_loop(app: AppHandle) {
    std::thread::spawn(move || {
        let mut tick: u64 = 0;
        let mut last_gpu: Option<GpuTelemetry> = None;
        loop {
            std::thread::sleep(Duration::from_secs(1));
            let state = app.state::<DashboardHostState>();
            let (poll_telemetry, poll_gpu) = match state.core.lock() {
                Ok(core) => (core.should_poll_telemetry(), core.should_poll_gpu()),
                Err(_) => (false, false),
            };
            if !poll_telemetry {
                last_gpu = None;
                continue;
            }
            tick = tick.wrapping_add(1);
            if poll_gpu && tick % 2 == 1 {
                if let Ok(gpu) = tauri::async_runtime::block_on(helper_bridge::fetch_gpu_telemetry()) {
                    last_gpu = Some(gpu);
                }
            }
            let snapshot = match state.sampler.lock() {
                Ok(mut sampler) => sampler.sample(if poll_gpu { last_gpu.clone() } else { None }),
                Err(_) => continue,
            };
            let _ = app.emit("dashboard://telemetry", &snapshot);
        }
    });
}
