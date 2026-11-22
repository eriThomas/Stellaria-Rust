#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app_catalog;
mod helper_bridge;
mod telemetry;
mod web_services;
mod win32_capture;
mod win32_shell;
mod win32_tasks;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::Serialize;
use stellaria_core::flags::FeatureFlags;
use stellaria_core::logging::StructuredLogger;
use stellaria_core::lyrics::LyricsResult;
use stellaria_core::monitors::{
    compute_top_dashboard_placement, monitor_at_point, MonitorInfo, PhysicalRect,
};
use stellaria_core::preview_cache::{PreviewCache, PreviewEntry};
use stellaria_core::settings::{DashboardSettings, DashboardSettingsPatch};
use stellaria_core::shell_state::{BarEdge, RecoveryReason, ShellSnapshot, ShellState};
use stellaria_core::state::{DashboardCore, DashboardSnapshot, DashboardTab, OpenTrigger, VisibilityEvent};
use stellaria_core::taskbar_model::{build_task_groups, PinStore, TaskGroup, WindowIdentity};
use stellaria_core::telemetry::{GpuTelemetry, MediaAction, MediaSnapshot, SystemInfo, TelemetrySnapshot};
use stellaria_core::weather::WeatherSnapshot;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, Runtime, State, WebviewUrl,
    WebviewWindowBuilder,
};
use windows_sys::Win32::Foundation::{HWND, POINT};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetCursorPos, SetWindowPos, ShowWindow, HWND_TOPMOST, SWP_NOACTIVATE, SWP_SHOWWINDOW,
    SW_SHOWNOACTIVATE,
};

use app_catalog::CatalogApp;
use telemetry::SystemMetricsSampler;
use web_services::WebServicesState;

const DASHBOARD_LABEL: &str = "dashboard";
const LAUNCHER_LABEL: &str = "launcher";
const DASH_LOGICAL_W: f64 = 1040.0;
const DASH_LOGICAL_H: f64 = 560.0;
const HOVER_TRIGGER_HALF_W: f64 = 450.0;
const HOVER_CLOSE_GRACE_MS: u64 = 180;
const HOVER_REOPEN_COOLDOWN_MS: u64 = 350;
const CLOSE_ANIMATION_MS: u64 = 340;

pub fn environment(name: &str) -> PathBuf {
    PathBuf::from(std::env::var_os(name).unwrap_or_default())
}

fn shell_data_dir() -> PathBuf {
    environment("LOCALAPPDATA").join("Stellaria-Shell")
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskbarStatePayload {
    pub revision: u64,
    pub session_generation: u64,
    pub groups: Vec<TaskGroup>,
    pub icons: HashMap<String, String>,
}

pub struct ShellAppState {
    pub shell: Mutex<ShellState>,
    pub dashboard: Mutex<DashboardCore>,
    pub sampler: Mutex<SystemMetricsSampler>,
    pub pins: Mutex<PinStore>,
    pub previews: Mutex<PreviewCache>,
    pub catalog: Mutex<Vec<CatalogApp>>,
    pub last_taskbar_payload: Mutex<Option<TaskbarStatePayload>>,
    pub logger: StructuredLogger,
    settings_path: PathBuf,
    pins_path: PathBuf,
    windows_taskbar_hidden: Mutex<bool>,
}

impl ShellAppState {
    fn initialize() -> Self {
        let dir = shell_data_dir();
        let session_generation = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(1);
        let settings_path = dir.join("settings.v1.json");
        let pins_path = dir.join("pins.v1.json");
        let log_path = dir.join("logs").join("shell.jsonl");

        let settings = DashboardSettings::load_or_default(&settings_path);
        let pins = PinStore::load_or_default(&pins_path);
        let logger = StructuredLogger::new(Some(log_path), session_generation);
        let flags = FeatureFlags::experimental();

        Self {
            shell: Mutex::new(ShellState::new(session_generation, true)),
            dashboard: Mutex::new(DashboardCore::new(session_generation, settings, flags)),
            sampler: Mutex::new(SystemMetricsSampler::default()),
            pins: Mutex::new(pins),
            previews: Mutex::new(PreviewCache::default()),
            catalog: Mutex::new(Vec::new()),
            last_taskbar_payload: Mutex::new(None),
            logger,
            settings_path,
            pins_path,
            windows_taskbar_hidden: Mutex::new(false),
        }
    }
}

fn collect_monitors<R: Runtime>(app: &AppHandle<R>) -> Vec<MonitorInfo> {
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

fn sync_bar_windows<R: Runtime>(app: &AppHandle<R>, snapshot: &ShellSnapshot) {
    if win32_shell::is_session_locked() {
        return;
    }

    let expected_labels: Vec<&str> = snapshot
        .monitors
        .iter()
        .map(|m| m.window_label.as_str())
        .collect();

    for (label, window) in app.webview_windows() {
        if label.starts_with("bar-") && !expected_labels.contains(&label.as_str()) {
            let _ = window.destroy();
        }
    }

    if !snapshot.synthetic_bars_enabled {
        for (label, window) in app.webview_windows() {
            if label.starts_with("bar-") {
                let _ = window.destroy();
            }
        }
        return;
    }

    for view in &snapshot.monitors {
        let label = &view.window_label;
        let p = view.bar_placement;
        let window = if let Some(existing) = app.get_webview_window(label) {
            existing
        } else {
            let url = format!("bar.html?monitor={label}");
            let Ok(created) = WebviewWindowBuilder::new(app, label, WebviewUrl::App(url.into()))
                .title("Stellaria Bar")
                .decorations(false)
                .shadow(false)
                .transparent(true)
                .resizable(false)
                .skip_taskbar(true)
                .always_on_top(true)
                .visible(false)
                .focused(false)
                .build()
            else {
                continue;
            };
            created
        };

        if let Ok(handle) = window.hwnd() {
            win32_shell::exclude_from_alt_tab(handle.0 as HWND);
        }

        let _ = window.set_position(PhysicalPosition::new(p.x, p.y));
        let _ = window.set_size(PhysicalSize::new(p.width, p.height));

        if view.monitor.fullscreen {
            let _ = window.hide();
        } else {
            if let Ok(handle) = window.hwnd() {
                unsafe {
                    ShowWindow(handle.0 as HWND, SW_SHOWNOACTIVATE);
                    SetWindowPos(
                        handle.0 as HWND,
                        HWND_TOPMOST,
                        p.x,
                        p.y,
                        p.width as i32,
                        p.height as i32,
                        SWP_NOACTIVATE | SWP_SHOWWINDOW,
                    );
                }
            }
            let _ = window.show();
        }
    }
}

fn reconcile_topology<R: Runtime>(app: &AppHandle<R>, reason: Option<RecoveryReason>) {
    if win32_shell::is_session_locked() {
        return;
    }
    let monitors = collect_monitors(app);
    let state = app.state::<ShellAppState>();
    let snapshot = {
        let Ok(mut shell) = state.shell.lock() else {
            return;
        };
        if let Some(r) = reason {
            shell.on_session_boundary(r);
            if let Ok(mut previews) = state.previews.lock() {
                previews.invalidate_generation(shell.session_generation());
            }
        }
        shell.reconcile_monitors(monitors);
        let mon_list = shell.monitors().to_vec();
        let snap = shell.snapshot();
        drop(shell);
        if let Ok(mut dash) = state.dashboard.lock() {
            if let Some(ev) = dash.update_monitors(mon_list) {
                let _ = app.emit("dashboard://visibility", &ev);
            }
        }
        snap
    };
    sync_bar_windows(app, &snapshot);
    let _ = app.emit("shell://snapshot", &snapshot);
}

fn ensure_dashboard_window<R: Runtime>(app: &AppHandle<R>) -> Result<tauri::WebviewWindow<R>, String> {
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
        .inner_size(DASH_LOGICAL_W, DASH_LOGICAL_H)
        .build()
        .map_err(|e| e.to_string())?;
    if let Ok(handle) = window.hwnd() {
        win32_shell::exclude_from_alt_tab(handle.0 as HWND);
    }
    Ok(window)
}

fn apply_dashboard_visibility<R: Runtime>(
    app: &AppHandle<R>,
    open: bool,
    monitor_id: Option<String>,
    trigger: OpenTrigger,
) -> Result<VisibilityEvent, String> {
    if win32_shell::is_session_locked() && open {
        return Err("session-locked".to_string());
    }
    let monitors = collect_monitors(app);
    let state = app.state::<ShellAppState>();
    let (event, target_monitor) = {
        let mut dash = state.dashboard.lock().map_err(|_| "busy".to_string())?;
        dash.update_monitors(monitors.clone());
        let event = dash.set_visibility(open, monitor_id, trigger)?;
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
            win32_shell::exclude_from_alt_tab(handle.0 as HWND);
        }
        if let Some(mon) = target_monitor {
            let placement = compute_top_dashboard_placement(&mon, DASH_LOGICAL_W, DASH_LOGICAL_H);
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
            let _ = window.show();
            let _ = window.set_focus();
        }
        let _ = app.emit("dashboard://visibility", &event);
    } else {
        let _ = app.emit("dashboard://visibility", &event);
        let app_clone = app.clone();
        let expected_rev = event.revision;
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(CLOSE_ANIMATION_MS));
            let still_closed = app_clone
                .state::<ShellAppState>()
                .dashboard
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

fn toggle_launcher_window<R: Runtime>(app: &AppHandle<R>) {
    if win32_shell::is_session_locked() {
        return;
    }
    if let Some(existing) = app.get_webview_window(LAUNCHER_LABEL) {
        if existing.is_visible().unwrap_or(false) {
            let _ = existing.hide();
            return;
        }
        let _ = existing.show();
        let _ = existing.set_focus();
        let _ = app.emit("launcher://opened", ());
        return;
    }

    let Ok(window) = WebviewWindowBuilder::new(app, LAUNCHER_LABEL, WebviewUrl::App("launcher.html".into()))
        .title("Stellaria Launcher")
        .decorations(false)
        .shadow(false)
        .transparent(true)
        .resizable(false)
        .skip_taskbar(true)
        .always_on_top(true)
        .center()
        .inner_size(640.0, 520.0)
        .build()
    else {
        return;
    };
    if let Ok(handle) = window.hwnd() {
        win32_shell::exclude_from_alt_tab(handle.0 as HWND);
    }
    let _ = window.show();
    let _ = window.set_focus();
}

fn build_and_broadcast_tasks<R: Runtime>(app: &AppHandle<R>) {
    let windows = win32_tasks::enumerate_windows();
    let state = app.state::<ShellAppState>();
    let pins = state.pins.lock().map(|p| p.pins.clone()).unwrap_or_default();
    let groups = build_task_groups(&windows, &pins, None, true);

    let mut icons = HashMap::new();
    for group in &groups {
        if !group.exe_path.is_empty() {
            if let Some(url) = win32_capture::extract_app_icon_data_url(&group.exe_path) {
                icons.insert(group.key.clone(), url);
            }
        }
    }

    let (revision, session_generation) = state
        .shell
        .lock()
        .map(|s| (s.revision(), s.session_generation()))
        .unwrap_or((1, 1));

    let payload = TaskbarStatePayload {
        revision,
        session_generation,
        groups,
        icons,
    };

    if let Ok(mut slot) = state.last_taskbar_payload.lock() {
        *slot = Some(payload.clone());
    }
    let _ = app.emit("shell://tasks", &payload);
}

fn start_background_loops(app: AppHandle) {
    // 1. Native Win32 task enumeration loop (1s interval, paused when locked)
    let app_tasks = app.clone();
    std::thread::spawn(move || loop {
        if !win32_shell::is_session_locked() {
            build_and_broadcast_tasks(&app_tasks);
        }
        std::thread::sleep(Duration::from_secs(1));
    });

    // 2. Visibility-gated dashboard telemetry loop (ZERO sampling while dashboard is hidden)
    let app_tel = app.clone();
    std::thread::spawn(move || {
        let mut tick: u64 = 0;
        let mut last_gpu: Option<GpuTelemetry> = None;
        loop {
            std::thread::sleep(Duration::from_secs(1));
            if win32_shell::is_session_locked() {
                continue;
            }
            let state = app_tel.state::<ShellAppState>();
            let (poll_tel, poll_gpu) = state
                .dashboard
                .lock()
                .map(|c| (c.should_poll_telemetry(), c.should_poll_gpu()))
                .unwrap_or((false, false));
            if !poll_tel {
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
            let _ = app_tel.emit("dashboard://telemetry", &snapshot);
        }
    });

    // 3. Top-edge hover auto-show / auto-hide watcher (paused when locked)
    let app_hover = app.clone();
    std::thread::spawn(move || {
        let mut last_closed_at = Instant::now() - Duration::from_secs(5);
        let mut outside_since: Option<Instant> = None;
        loop {
            let (is_open, trigger, active_mon) = app_hover
                .state::<ShellAppState>()
                .dashboard
                .lock()
                .map(|c| {
                    let v = c.visibility();
                    (v.open, v.trigger, v.monitor_id.clone())
                })
                .unwrap_or((false, None, None));

            std::thread::sleep(Duration::from_millis(if is_open { 50 } else { 80 }));
            if win32_shell::is_session_locked() {
                continue;
            }

            let mut pt = POINT { x: 0, y: 0 };
            if unsafe { GetCursorPos(&mut pt) } == 0 {
                continue;
            }
            let (cx, cy) = (pt.x, pt.y);

            if !is_open {
                outside_since = None;
                if last_closed_at.elapsed().as_millis() < HOVER_REOPEN_COOLDOWN_MS as u128 {
                    continue;
                }
                let monitors = collect_monitors(&app_hover);
                let Some(mon) = monitor_at_point(&monitors, cx, cy) else {
                    continue;
                };
                let scale = mon.scale_factor.max(1.0);
                let center_x = mon.bounds.left + mon.bounds.width() / 2;
                let half_zone = (HOVER_TRIGGER_HALF_W * scale).round() as i32;
                if cy <= mon.bounds.top + 6 && cy >= mon.bounds.top - 2 && (cx - center_x).abs() <= half_zone {
                    let own_hwnd = app_hover
                        .get_webview_window(DASHBOARD_LABEL)
                        .and_then(|w| w.hwnd().ok())
                        .map(|h| h.0 as HWND)
                        .unwrap_or(std::ptr::null_mut());
                    if !win32_shell::is_true_fullscreen(own_hwnd) {
                        let _ = apply_dashboard_visibility(&app_hover, true, Some(mon.id.clone()), OpenTrigger::Hover);
                    }
                }
            } else if trigger == Some(OpenTrigger::Hover) {
                let monitors = collect_monitors(&app_hover);
                let mon = active_mon
                    .as_deref()
                    .and_then(|id| monitors.iter().find(|m| m.id == id))
                    .or_else(|| monitor_at_point(&monitors, cx, cy));
                let Some(mon) = mon else {
                    continue;
                };
                let p = compute_top_dashboard_placement(mon, DASH_LOGICAL_W, DASH_LOGICAL_H);
                let margin = (24.0 * mon.scale_factor.max(1.0)).round() as i32;
                let inside = cx >= p.x - margin
                    && cx <= p.x + p.width as i32 + margin
                    && cy >= p.y - 4
                    && cy <= p.y + p.height as i32;
                if inside {
                    outside_since = None;
                } else {
                    let since = outside_since.get_or_insert_with(Instant::now);
                    if since.elapsed().as_millis() >= HOVER_CLOSE_GRACE_MS as u128 {
                        let _ = apply_dashboard_visibility(&app_hover, false, Some(mon.id.clone()), OpenTrigger::Hover);
                        last_closed_at = Instant::now();
                        outside_since = None;
                    }
                }
            }
        }
    });

    // 4. Native Win32 lock/unlock + sleep/resume + display + Explorer restart watcher
    let app_rec = app.clone();
    win32_shell::start_session_watcher(move |reason| {
        reconcile_topology(&app_rec, Some(reason));
        build_and_broadcast_tasks(&app_rec);
    });

    // 5. Background Start Menu app catalog scan
    let app_cat = app;
    std::thread::spawn(move || {
        let apps = app_catalog::scan_start_menu_apps();
        if let Ok(mut slot) = app_cat.state::<ShellAppState>().catalog.lock() {
            *slot = apps;
        }
    });
}

// ---------- Typed Tauri Commands ----------

#[tauri::command]
fn dashboard_get_snapshot(app: AppHandle, state: State<'_, ShellAppState>) -> Result<DashboardSnapshot, String> {
    let monitors = collect_monitors(&app);
    let mut dash = state.dashboard.lock().map_err(|_| "busy".to_string())?;
    dash.update_monitors(monitors);
    Ok(dash.snapshot())
}

#[tauri::command]
fn dashboard_set_visibility(
    app: AppHandle,
    open: bool,
    monitor_id: Option<String>,
    trigger: OpenTrigger,
) -> Result<VisibilityEvent, String> {
    apply_dashboard_visibility(&app, open, monitor_id, trigger)
}

#[tauri::command]
fn dashboard_set_active_tab(
    app: AppHandle,
    state: State<'_, ShellAppState>,
    tab: DashboardTab,
) -> Result<VisibilityEvent, String> {
    let event = state
        .dashboard
        .lock()
        .map_err(|_| "busy".to_string())?
        .set_active_tab(tab);
    let _ = app.emit("dashboard://visibility", &event);
    Ok(event)
}

#[tauri::command]
fn dashboard_update_settings(
    app: AppHandle,
    state: State<'_, ShellAppState>,
    patch: DashboardSettingsPatch,
) -> Result<DashboardSettings, String> {
    let (revision, updated) = state
        .dashboard
        .lock()
        .map_err(|_| "busy".to_string())?
        .update_settings(patch)?;
    updated.save_atomic(&state.settings_path)?;
    let _ = app.emit("dashboard://settings", serde_json::json!({ "revision": revision, "settings": &updated }));
    Ok(updated)
}

#[tauri::command]
async fn dashboard_get_sysinfo() -> Result<SystemInfo, String> {
    helper_bridge::fetch_system_info().await.or_else(|_| {
        Ok(SystemInfo {
            os: "Windows 11".into(),
            cpu: std::env::var("PROCESSOR_IDENTIFIER").unwrap_or_else(|_| "CPU".into()),
            gpu: None,
            user: std::env::var("USERNAME").unwrap_or_else(|_| "User".into()),
            picture_base64: None,
        })
    })
}

#[tauri::command]
async fn dashboard_get_telemetry(state: State<'_, ShellAppState>) -> Result<TelemetrySnapshot, String> {
    let want_gpu = state
        .dashboard
        .lock()
        .map(|c| c.should_poll_gpu())
        .unwrap_or(false);
    let gpu = if want_gpu {
        helper_bridge::fetch_gpu_telemetry().await.ok()
    } else {
        None
    };
    let mut sampler = state.sampler.lock().map_err(|_| "busy".to_string())?;
    Ok(sampler.sample(gpu))
}

#[tauri::command]
async fn dashboard_get_media() -> Result<MediaSnapshot, String> {
    helper_bridge::fetch_media_snapshot()
        .await
        .or_else(|_| Ok(MediaSnapshot::default()))
}

#[tauri::command]
async fn dashboard_media_action(action: MediaAction) -> Result<(), String> {
    helper_bridge::send_media_action(action).await
}

#[tauri::command]
async fn dashboard_refresh_weather(
    state: State<'_, ShellAppState>,
    web: State<'_, WebServicesState>,
    force: bool,
) -> Result<Option<WeatherSnapshot>, String> {
    let enabled = state
        .dashboard
        .lock()
        .map(|c| c.settings().weather_enabled)
        .unwrap_or(true);
    web_services::fetch_weather(&web, enabled, force).await
}

#[tauri::command]
async fn dashboard_get_lyrics(
    state: State<'_, ShellAppState>,
    web: State<'_, WebServicesState>,
    artist: String,
    title: String,
    duration_secs: Option<f64>,
) -> Result<LyricsResult, String> {
    let enabled = state
        .dashboard
        .lock()
        .map(|c| c.settings().lyrics_enabled)
        .unwrap_or(true);
    if !enabled {
        return Ok(LyricsResult {
            key: String::new(),
            lines: Vec::new(),
        });
    }
    web_services::search_lyrics(&web, artist, title, duration_secs).await
}

#[tauri::command]
fn shell_get_snapshot(app: AppHandle, state: State<'_, ShellAppState>) -> Result<ShellSnapshot, String> {
    let mut shell = state.shell.lock().map_err(|_| "busy".to_string())?;
    if shell.monitors().is_empty() {
        shell.reconcile_monitors(collect_monitors(&app));
    }
    Ok(shell.snapshot())
}

#[tauri::command]
fn taskbar_get_state(app: AppHandle, state: State<'_, ShellAppState>) -> Result<TaskbarStatePayload, String> {
    if let Some(cached) = state.last_taskbar_payload.lock().map_err(|_| "busy".to_string())?.clone() {
        return Ok(cached);
    }
    build_and_broadcast_tasks(&app);
    state
        .last_taskbar_payload
        .lock()
        .map_err(|_| "busy".to_string())?
        .clone()
        .ok_or_else(|| "empty".to_string())
}

#[tauri::command]
fn taskbar_activate_window(app: AppHandle, hwnd: u64, pid: u32, allow_minimize: bool) -> Result<(), String> {
    let res = win32_tasks::activate_or_toggle_window(hwnd, pid, allow_minimize);
    build_and_broadcast_tasks(&app);
    res
}

#[tauri::command]
fn taskbar_close_window(app: AppHandle, hwnd: u64, pid: u32) -> Result<(), String> {
    let res = win32_tasks::close_window_graceful(hwnd, pid);
    build_and_broadcast_tasks(&app);
    res
}

#[tauri::command]
fn taskbar_toggle_pin(
    app: AppHandle,
    state: State<'_, ShellAppState>,
    exe_path: String,
    exe_name: String,
    display_name: Option<String>,
) -> Result<bool, String> {
    let pinned = {
        let mut pins = state.pins.lock().map_err(|_| "busy".to_string())?;
        let now_pinned = pins.toggle_pin(&exe_path, &exe_name, display_name.as_deref());
        pins.save_atomic(&state.pins_path)?;
        now_pinned
    };
    build_and_broadcast_tasks(&app);
    Ok(pinned)
}

#[tauri::command]
fn taskbar_launch_app(path: String) -> Result<(), String> {
    win32_tasks::launch_app_safe(&path)
}

#[tauri::command]
async fn taskbar_capture_preview(
    state: State<'_, ShellAppState>,
    hwnd: u64,
    pid: u32,
) -> Result<PreviewEntry, String> {
    let gen = state
        .shell
        .lock()
        .map(|s| s.session_generation())
        .unwrap_or(1);
    let identity = WindowIdentity {
        hwnd,
        pid,
        session_generation: gen,
    };
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    if let Ok(cache) = state.previews.lock() {
        if let Some(hit) = cache.get(identity, now_ms, 1500) {
            return Ok(hit);
        }
    }
    let entry = tauri::async_runtime::spawn_blocking(move || {
        win32_capture::capture_window_preview(hwnd, pid, gen)
    })
    .await
    .map_err(|e| e.to_string())?;
    if let Ok(mut cache) = state.previews.lock() {
        cache.insert(entry.clone());
    }
    Ok(entry)
}

#[tauri::command]
fn launcher_get_apps(state: State<'_, ShellAppState>) -> Result<Vec<CatalogApp>, String> {
    let apps = state.catalog.lock().map_err(|_| "busy".to_string())?.clone();
    Ok(apps)
}

#[tauri::command]
fn launcher_toggle(app: AppHandle) {
    toggle_launcher_window(&app);
}

#[tauri::command]
fn launcher_hide(app: AppHandle) {
    if let Some(w) = app.get_webview_window(LAUNCHER_LABEL) {
        let _ = w.hide();
    }
}

#[tauri::command]
fn shell_set_bar_edge(app: AppHandle, state: State<'_, ShellAppState>, edge: BarEdge) -> Result<(), String> {
    let changed = state.shell.lock().map_err(|_| "busy".to_string())?.set_bar_edge(edge);
    if changed {
        reconcile_topology(&app, None);
    }
    Ok(())
}

fn create_tray<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let menu = Menu::new(app)?;
    menu.append(&MenuItem::with_id(app, "dashboard", "Open Dashboard", true, None::<&str>)?)?;
    menu.append(&MenuItem::with_id(app, "launcher", "Open App Launcher", true, None::<&str>)?)?;
    menu.append(&MenuItem::with_id(app, "toggle-bars", "Toggle Rust Taskbars", true, None::<&str>)?)?;
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    menu.append(&MenuItem::with_id(app, "toggle-win-bar", "Hide / Show Windows Taskbar", true, None::<&str>)?)?;
    menu.append(&MenuItem::with_id(app, "restore-shell", "Restore Windows Taskbar", true, None::<&str>)?)?;
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    menu.append(&MenuItem::with_id(app, "quit", "Quit Stellaria Shell", true, None::<&str>)?)?;

    let mut builder = TrayIconBuilder::with_id("stellaria-shell-tray")
        .tooltip("Stellaria Shell (Rust)")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "dashboard" => {
                let open = app
                    .state::<ShellAppState>()
                    .dashboard
                    .lock()
                    .map(|c| c.visibility().open)
                    .unwrap_or(false);
                let _ = apply_dashboard_visibility(app, !open, None, OpenTrigger::Tray);
            }
            "launcher" => toggle_launcher_window(app),
            "toggle-bars" => {
                let state = app.state::<ShellAppState>();
                let next = {
                    let Ok(mut shell) = state.shell.lock() else {
                        return;
                    };
                    let enabled = !shell.synthetic_bars_enabled();
                    shell.set_synthetic_bars_enabled(enabled);
                    shell.snapshot()
                };
                sync_bar_windows(app, &next);
            }
            "toggle-win-bar" => {
                let state = app.state::<ShellAppState>();
                if let Ok(mut hidden) = state.windows_taskbar_hidden.lock() {
                    *hidden = !*hidden;
                    win32_shell::set_windows_taskbar_visible(!*hidden);
                };
            }
            "restore-shell" => {
                if let Ok(mut hidden) = app.state::<ShellAppState>().windows_taskbar_hidden.lock() {
                    *hidden = false;
                }
                win32_shell::restore_windows_shell();
            }
            "quit" => {
                win32_shell::restore_windows_shell();
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                let open = app
                    .state::<ShellAppState>()
                    .dashboard
                    .lock()
                    .map(|c| c.visibility().open)
                    .unwrap_or(false);
                let _ = apply_dashboard_visibility(app, !open, None, OpenTrigger::Tray);
            }
        });

    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}

fn main() {
    win32_shell::install_shell_restore_panic_hook();
    let _restore_guard = win32_shell::ShellRestoreGuard;

    let replace_taskbar = std::env::args().any(|arg| arg == "--replace-taskbar");
    let app_state = ShellAppState::initialize();

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            let _ = apply_dashboard_visibility(app, true, None, OpenTrigger::Tray);
        }))
        .manage(app_state)
        .manage(WebServicesState::default())
        .setup(move |app| {
            create_tray(app.handle())?;
            if replace_taskbar {
                if let Ok(mut hidden) = app.state::<ShellAppState>().windows_taskbar_hidden.lock() {
                    *hidden = true;
                }
                win32_shell::set_windows_taskbar_visible(false);
            }
            reconcile_topology(app.handle(), None);
            start_background_loops(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            dashboard_get_snapshot,
            dashboard_set_visibility,
            dashboard_set_active_tab,
            dashboard_update_settings,
            dashboard_get_sysinfo,
            dashboard_get_telemetry,
            dashboard_get_media,
            dashboard_media_action,
            dashboard_refresh_weather,
            dashboard_get_lyrics,
            shell_get_snapshot,
            shell_set_bar_edge,
            taskbar_get_state,
            taskbar_activate_window,
            taskbar_close_window,
            taskbar_toggle_pin,
            taskbar_launch_app,
            taskbar_capture_preview,
            launcher_get_apps,
            launcher_toggle,
            launcher_hide
        ])
        .run(tauri::generate_context!())
        .expect("Stellaria Shell could not start");
}
