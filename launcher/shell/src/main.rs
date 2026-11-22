#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app_catalog;
mod helper_bridge;
mod telemetry;
mod web_services;
mod win32_audio;
mod win32_capture;
mod win32_net;
mod win32_shell;
mod win32_sysinfo;
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
use stellaria_core::taskbar_model::{build_task_groups, PinStore, StableTaskOrder, TaskGroup, WindowIdentity};
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
const PREVIEW_LABEL: &str = "preview";
const SESSION_LABEL: &str = "session";
const POPOUT_LABEL: &str = "popout";
const OSD_LABEL: &str = "osd";
const ISLAND_LABEL: &str = "island";

static OSD_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
static ISLAND_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
static ESTIMATED_VOLUME: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(50);
static ESTIMATED_MUTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
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
    pub task_order: Mutex<StableTaskOrder>,
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
            task_order: Mutex::new(StableTaskOrder::default()),
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
            if let Ok(handle) = window.hwnd() {
                win32_shell::unregister_appbar(handle.0 as HWND);
            }
            let _ = window.destroy();
        }
    }

    if !snapshot.synthetic_bars_enabled {
        win32_shell::unregister_all_appbars();
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
            if let Ok(handle) = window.hwnd() {
                win32_shell::unregister_appbar(handle.0 as HWND);
            }
            win32_shell::restore_monitor_work_area(&view.monitor.id);
            let _ = window.hide();
        } else {
            if let Ok(handle) = window.hwnd() {
                let hwnd = handle.0 as HWND;
                unsafe {
                    ShowWindow(hwnd, SW_SHOWNOACTIVATE);
                    SetWindowPos(
                        hwnd,
                        HWND_TOPMOST,
                        p.x,
                        p.y,
                        p.width as i32,
                        p.height as i32,
                        SWP_NOACTIVATE | SWP_SHOWWINDOW,
                    );
                }
                win32_shell::reserve_monitor_work_area(
                    &view.monitor,
                    snapshot.bar_edge,
                    p.width as i32,
                    p.height as i32,
                );
                win32_shell::register_and_reserve_appbar(
                    hwnd,
                    snapshot.bar_edge,
                    p.x,
                    p.y,
                    p.width as i32,
                    p.height as i32,
                );
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
    let mut groups = build_task_groups(&windows, &pins, None, true);
    if let Ok(mut order) = state.task_order.lock() {
        order.stabilize(&mut groups);
    }

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
    // Also passively caches the active foreground window's thumbnail every 4s so minimized windows keep a live preview!
    let app_tasks = app.clone();
    std::thread::spawn(move || {
        let mut tick: u64 = 0;
        loop {
            if !win32_shell::is_session_locked() {
                build_and_broadcast_tasks(&app_tasks);
                tick = tick.wrapping_add(1);
                if tick % 4 == 1 {
                    let state = app_tasks.state::<ShellAppState>();
                    let active_win = state
                        .last_taskbar_payload
                        .lock()
                        .ok()
                        .and_then(|p| {
                            p.as_ref().and_then(|payload| {
                                payload
                                    .groups
                                    .iter()
                                    .flat_map(|g| g.windows.iter())
                                    .find(|w| w.active && !w.minimized)
                                    .map(|w| (w.hwnd, w.pid, payload.session_generation))
                            })
                        });
                    if let Some((hwnd, pid, gen)) = active_win {
                        let entry = win32_capture::capture_window_preview(hwnd, pid, gen);
                        if entry.png_base64.is_some() {
                            if let Ok(mut cache) = state.previews.lock() {
                                cache.insert(entry);
                            }
                        }
                    }
                }

                // Per-monitor fullscreen auto-hide reconciliation (only hides the affected monitor's bar)
                let own_hwnds: Vec<HWND> = app_tasks
                    .webview_windows()
                    .values()
                    .filter_map(|w| w.hwnd().ok().map(|h| h.0 as HWND))
                    .collect();
                let monitors_copy = app_tasks
                    .state::<ShellAppState>()
                    .shell
                    .lock()
                    .map(|s| s.monitors().to_vec())
                    .unwrap_or_default();
                if !monitors_copy.is_empty() {
                    let fs_ids = win32_shell::detect_fullscreen_monitors(&monitors_copy, &own_hwnds);
                    let maybe_snapshot = app_tasks
                        .state::<ShellAppState>()
                        .shell
                        .lock()
                        .ok()
                        .and_then(|mut s| {
                            if s.reconcile_fullscreen_monitors(&fs_ids) {
                                Some(s.snapshot())
                            } else {
                                None
                            }
                        });
                    if let Some(snap) = maybe_snapshot {
                        sync_bar_windows(&app_tasks, &snap);
                        let _ = app_tasks.emit("shell://snapshot", &snap);
                    }
                }
            }
            std::thread::sleep(Duration::from_secs(1));
        }
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
                if let Some(gpu) = win32_sysinfo::query_gpu_telemetry() {
                    last_gpu = Some(gpu);
                } else if let Ok(gpu) = tauri::async_runtime::block_on(helper_bridge::fetch_gpu_telemetry()) {
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
async fn dashboard_set_visibility(
    app: AppHandle,
    state: State<'_, ShellAppState>,
    open: bool,
    monitor_id: Option<String>,
    trigger: OpenTrigger,
    tab: Option<DashboardTab>,
) -> Result<VisibilityEvent, String> {
    if let Some(target_tab) = tab {
        if let Ok(mut dash) = state.dashboard.lock() {
            let _ = dash.set_active_tab(target_tab);
        }
    }
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
    let mut native = tauri::async_runtime::spawn_blocking(win32_sysinfo::query_system_info)
        .await
        .map_err(|e| e.to_string())?;
    if let Ok(helper_info) = helper_bridge::fetch_system_info().await {
        if native.picture_base64.is_none() {
            native.picture_base64 = helper_info.picture_base64;
        }
        if native.gpu.is_none() {
            native.gpu = helper_info.gpu;
        }
    }
    Ok(native)
}

#[tauri::command]
async fn dashboard_get_telemetry(state: State<'_, ShellAppState>) -> Result<TelemetrySnapshot, String> {
    let want_gpu = state
        .dashboard
        .lock()
        .map(|c| c.should_poll_gpu())
        .unwrap_or(false);
    let gpu = if want_gpu {
        let native_gpu = tauri::async_runtime::spawn_blocking(win32_sysinfo::query_gpu_telemetry)
            .await
            .ok()
            .flatten();
        match native_gpu {
            Some(g) => Some(g),
            None => helper_bridge::fetch_gpu_telemetry().await.ok(),
        }
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
    if entry.png_base64.is_none() {
        // Window is minimized or protected: return last known live frame from this session if available!
        if let Ok(cache) = state.previews.lock() {
            if let Some(mut frozen) = cache.get_last_bitmap(identity) {
                frozen.fallback_reason = Some("minimized-cached".into());
                return Ok(frozen);
            }
        }
    } else if let Ok(mut cache) = state.previews.lock() {
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
async fn launcher_toggle(app: AppHandle) {
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

#[tauri::command]
async fn preview_show(
    app: AppHandle,
    state: State<'_, ShellAppState>,
    monitor_id: Option<String>,
    group_key: String,
    anchor_y: f64,
    mode: Option<String>,
) -> Result<(), String> {
    if win32_shell::is_session_locked() {
        return Ok(());
    }
    let mode_str = mode.unwrap_or_else(|| "preview".to_string());
    let payload = state
        .last_taskbar_payload
        .lock()
        .map_err(|_| "busy".to_string())?
        .clone()
        .ok_or_else(|| "no-tasks".to_string())?;
    let group = payload
        .groups
        .into_iter()
        .find(|g| g.key == group_key)
        .ok_or_else(|| "group-not-found".to_string())?;
    if mode_str == "preview" && group.windows.is_empty() {
        return Ok(());
    }
    let icon_url = payload.icons.get(&group.key).cloned();

    let monitors = collect_monitors(&app);
    let mon = monitor_id
        .as_deref()
        .and_then(|id| monitors.iter().find(|m| m.id == id))
        .or_else(|| monitors.iter().find(|m| m.primary))
        .or_else(|| monitors.first())
        .cloned();

    let window = if let Some(existing) = app.get_webview_window(PREVIEW_LABEL) {
        existing
    } else {
        WebviewWindowBuilder::new(&app, PREVIEW_LABEL, WebviewUrl::App("preview.html".into()))
            .title("Stellaria Preview")
            .decorations(false)
            .shadow(false)
            .transparent(true)
            .resizable(false)
            .skip_taskbar(true)
            .always_on_top(true)
            .visible(false)
            .focused(false)
            .build()
            .map_err(|e| e.to_string())?
    };

    if let Ok(handle) = window.hwnd() {
        win32_shell::exclude_from_alt_tab(handle.0 as HWND);
    }

    if let Some(m) = mon {
        let scale = m.scale_factor.max(1.0);
        let (logical_w, logical_h) = if mode_str == "context" {
            (220.0, 170.0)
        } else {
            let cols = group.windows.len().clamp(1, 3) as f64;
            let rows = (group.windows.len().div_ceil(3)).clamp(1, 2) as f64;
            (cols * 244.0 + 24.0, rows * 170.0 + 48.0)
        };
        let width = (logical_w * scale).round() as i32;
        let height = (logical_h * scale).round() as i32;
        let bar_w = (54.0 * scale).round() as i32;
        let x = (m.bounds.left + bar_w).clamp(m.bounds.left, (m.bounds.right - width).max(m.bounds.left));
        let desired_y = m.bounds.top + (anchor_y * scale).round() as i32 - height / 2;
        let y = desired_y.clamp(m.bounds.top + 8, (m.bounds.bottom - height - 8).max(m.bounds.top));

        let _ = window.set_position(PhysicalPosition::new(x, y));
        let _ = window.set_size(PhysicalSize::new(width.max(180) as u32, height.max(140) as u32));
        if let Ok(handle) = window.hwnd() {
            unsafe {
                ShowWindow(handle.0 as HWND, SW_SHOWNOACTIVATE);
                SetWindowPos(
                    handle.0 as HWND,
                    HWND_TOPMOST,
                    x,
                    y,
                    width.max(180),
                    height.max(140),
                    SWP_NOACTIVATE | SWP_SHOWWINDOW,
                );
            }
        }
    }

    let _ = window.show();
    let _ = app.emit(
        "preview://show",
        serde_json::json!({ "group": group, "iconUrl": icon_url, "mode": mode_str }),
    );
    Ok(())
}

#[tauri::command]
fn preview_hide(app: AppHandle) {
    if let Some(w) = app.get_webview_window(PREVIEW_LABEL) {
        let _ = w.hide();
    }
}

#[tauri::command]
fn shell_set_keep_awake(enabled: bool) -> Result<(), String> {
    #[link(name = "kernel32")]
    extern "system" {
        fn SetThreadExecutionState(es_flags: u32) -> u32;
    }
    const ES_CONTINUOUS: u32 = 0x8000_0000;
    const ES_SYSTEM_REQUIRED: u32 = 0x0000_0001;
    const ES_DISPLAY_REQUIRED: u32 = 0x0000_0002;
    let flags = if enabled {
        ES_CONTINUOUS | ES_SYSTEM_REQUIRED | ES_DISPLAY_REQUIRED
    } else {
        ES_CONTINUOUS
    };
    unsafe {
        SetThreadExecutionState(flags);
    }
    Ok(())
}

#[tauri::command]
async fn dashboard_toggle(
    app: AppHandle,
    state: State<'_, ShellAppState>,
    monitor_id: Option<String>,
    tab: Option<DashboardTab>,
) -> Result<VisibilityEvent, String> {
    let (currently_open, current_tab) = state
        .dashboard
        .lock()
        .map(|d| (d.visibility().open, d.visibility().active_tab))
        .unwrap_or((false, DashboardTab::Dash));

    if currently_open {
        if let Some(requested_tab) = tab {
            if requested_tab != current_tab {
                let event = state
                    .dashboard
                    .lock()
                    .map_err(|_| "busy".to_string())?
                    .set_active_tab(requested_tab);
                let _ = app.emit("dashboard://visibility", &event);
                return Ok(event);
            }
        }
        return apply_dashboard_visibility(&app, false, monitor_id, OpenTrigger::Click);
    }

    if let Some(requested_tab) = tab {
        if let Ok(mut dash) = state.dashboard.lock() {
            let _ = dash.set_active_tab(requested_tab);
        }
    }
    apply_dashboard_visibility(&app, true, monitor_id, OpenTrigger::Click)
}

#[tauri::command]
async fn session_toggle(app: AppHandle, monitor_id: Option<String>) -> Result<(), String> {
    if win32_shell::is_session_locked() {
        return Ok(());
    }
    if let Some(existing) = app.get_webview_window(SESSION_LABEL) {
        if existing.is_visible().unwrap_or(false) {
            let _ = app.emit("session://visibility", false);
            let w = existing.clone();
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(320));
                let _ = w.hide();
            });
            return Ok(());
        }
        let monitors = collect_monitors(&app);
        if let Some(m) = monitor_id
            .as_deref()
            .and_then(|id| monitors.iter().find(|m| m.id == id))
            .or_else(|| monitors.iter().find(|m| m.primary))
            .or_else(|| monitors.first())
        {
            let scale = m.scale_factor.max(1.0);
            let w = (170.0 * scale).round() as i32;
            let h = (590.0 * scale).round() as i32;
            let x = m.bounds.right - w;
            let y = m.bounds.top + (m.bounds.height() - h) / 2;
            let _ = existing.set_position(PhysicalPosition::new(x, y));
            let _ = existing.set_size(PhysicalSize::new(w as u32, h as u32));
            if let Ok(handle) = existing.hwnd() {
                unsafe {
                    SetWindowPos(handle.0 as HWND, HWND_TOPMOST, x, y, w, h, SWP_SHOWWINDOW);
                }
            }
        }
        let _ = existing.show();
        let _ = existing.set_focus();
        let _ = app.emit("session://visibility", true);
        return Ok(());
    }

    let window = WebviewWindowBuilder::new(&app, SESSION_LABEL, WebviewUrl::App("session.html".into()))
        .title("Stellaria Session")
        .decorations(false)
        .shadow(false)
        .transparent(true)
        .resizable(false)
        .skip_taskbar(true)
        .always_on_top(true)
        .visible(false)
        .focused(false)
        .build()
        .map_err(|e| e.to_string())?;

    if let Ok(handle) = window.hwnd() {
        win32_shell::exclude_from_alt_tab(handle.0 as HWND);
    }

    let monitors = collect_monitors(&app);
    if let Some(m) = monitor_id
        .as_deref()
        .and_then(|id| monitors.iter().find(|m| m.id == id))
        .or_else(|| monitors.iter().find(|m| m.primary))
        .or_else(|| monitors.first())
    {
        let scale = m.scale_factor.max(1.0);
        let w = (170.0 * scale).round() as i32;
        let h = (590.0 * scale).round() as i32;
        let x = m.bounds.right - w;
        let y = m.bounds.top + (m.bounds.height() - h) / 2;
        let _ = window.set_position(PhysicalPosition::new(x, y));
        let _ = window.set_size(PhysicalSize::new(w as u32, h as u32));
        if let Ok(handle) = window.hwnd() {
            unsafe {
                SetWindowPos(handle.0 as HWND, HWND_TOPMOST, x, y, w, h, SWP_SHOWWINDOW);
            }
        }
    }
    let _ = window.show();
    let _ = window.set_focus();
    let _ = app.emit("session://visibility", true);
    Ok(())
}

#[tauri::command]
fn session_hide(app: AppHandle) {
    if let Some(w) = app.get_webview_window(SESSION_LABEL) {
        let _ = app.emit("session://visibility", false);
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(320));
            let _ = w.hide();
        });
    }
}

#[tauri::command]
fn shell_open_settings(page: String) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    const ALLOWED: [&str; 4] = ["sound", "network-status", "regionlanguage", "notifications"];
    if !ALLOWED.contains(&page.as_str()) {
        return Err("refused".to_string());
    }
    let uri = format!("ms-settings:{page}");
    std::process::Command::new("rundll32.exe")
        .args(["url.dll,FileProtocolHandler", &uri])
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn shell_get_master_audio() -> Result<win32_audio::MasterAudioState, String> {
    tauri::async_runtime::spawn_blocking(win32_audio::get_master_audio)
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn shell_set_master_volume(app: AppHandle, volume: u32) -> Result<win32_audio::MasterAudioState, String> {
    let state = tauri::async_runtime::spawn_blocking(move || win32_audio::set_master_volume(volume))
        .await
        .map_err(|e| e.to_string())??;
    ESTIMATED_VOLUME.store(state.volume, std::sync::atomic::Ordering::Relaxed);
    ESTIMATED_MUTED.store(state.muted, std::sync::atomic::Ordering::Relaxed);
    let _ = show_osd_window(&app, "volume", state.volume, state.muted);
    Ok(state)
}

#[tauri::command]
async fn shell_volume_action(app: AppHandle, action: String) -> Result<(), String> {
    let native_res = tauri::async_runtime::spawn_blocking(move || match action.as_str() {
        "mute" => win32_audio::toggle_master_mute(),
        "down" => win32_audio::step_master_volume(-2),
        "up" => win32_audio::step_master_volume(2),
        _ => Err("unknown-volume-action".to_string()),
    })
    .await
    .map_err(|e| e.to_string())?;

    if let Ok(state) = native_res {
        ESTIMATED_VOLUME.store(state.volume, std::sync::atomic::Ordering::Relaxed);
        ESTIMATED_MUTED.store(state.muted, std::sync::atomic::Ordering::Relaxed);
        let _ = show_osd_window(&app, "volume", state.volume, state.muted);
        return Ok(());
    }
    Err("volume-action-failed".to_string())
}

fn show_osd_window<R: Runtime>(app: &AppHandle<R>, kind: &str, value: u32, muted: bool) -> Result<(), String> {
    if win32_shell::is_session_locked() {
        return Ok(());
    }
    let window = if let Some(existing) = app.get_webview_window(OSD_LABEL) {
        existing
    } else {
        WebviewWindowBuilder::new(app, OSD_LABEL, WebviewUrl::App("osd.html".into()))
            .title("Stellaria OSD")
            .decorations(false)
            .shadow(false)
            .transparent(true)
            .resizable(false)
            .skip_taskbar(true)
            .always_on_top(true)
            .visible(false)
            .focused(false)
            .build()
            .map_err(|e| e.to_string())?
    };

    if let Ok(handle) = window.hwnd() {
        win32_shell::exclude_from_alt_tab(handle.0 as HWND);
    }

    let monitors = collect_monitors(app);
    let mut pt = POINT { x: 0, y: 0 };
    let cursor_mon = if unsafe { GetCursorPos(&mut pt) } != 0 {
        monitor_at_point(&monitors, pt.x, pt.y)
    } else {
        None
    };
    if let Some(m) = cursor_mon
        .or_else(|| monitors.iter().find(|m| m.primary))
        .or_else(|| monitors.first())
    {
        let scale = m.scale_factor.max(1.0);
        let w = (92.0 * scale).round() as i32;
        let h = (260.0 * scale).round() as i32;
        let x = m.bounds.right - w;
        let y = m.bounds.top + (m.bounds.height() - h) / 2;
        let _ = window.set_position(PhysicalPosition::new(x, y));
        let _ = window.set_size(PhysicalSize::new(w as u32, h as u32));
        if let Ok(handle) = window.hwnd() {
            unsafe {
                ShowWindow(handle.0 as HWND, SW_SHOWNOACTIVATE);
                SetWindowPos(
                    handle.0 as HWND,
                    HWND_TOPMOST,
                    x,
                    y,
                    w,
                    h,
                    SWP_NOACTIVATE | SWP_SHOWWINDOW,
                );
            }
        }
    }

    let _ = app.emit(
        "osd://show",
        serde_json::json!({ "kind": kind, "value": value, "muted": muted }),
    );
    let seq = OSD_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
    let app_clone = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(2000));
        if OSD_SEQ.load(std::sync::atomic::Ordering::Relaxed) == seq {
            let _ = app_clone.emit("osd://hide", ());
            std::thread::sleep(Duration::from_millis(280));
            if OSD_SEQ.load(std::sync::atomic::Ordering::Relaxed) == seq {
                if let Some(w) = app_clone.get_webview_window(OSD_LABEL) {
                    let _ = w.hide();
                }
            }
        }
    });
    Ok(())
}

#[tauri::command]
async fn island_show_toast(
    app: AppHandle,
    label: String,
    title: String,
    subtitle: Option<String>,
    icon: Option<String>,
) -> Result<(), String> {
    if win32_shell::is_session_locked() {
        return Ok(());
    }
    let window = if let Some(existing) = app.get_webview_window(ISLAND_LABEL) {
        existing
    } else {
        WebviewWindowBuilder::new(&app, ISLAND_LABEL, WebviewUrl::App("island.html".into()))
            .title("Stellaria Island")
            .decorations(false)
            .shadow(false)
            .transparent(true)
            .resizable(false)
            .skip_taskbar(true)
            .always_on_top(true)
            .visible(false)
            .focused(false)
            .build()
            .map_err(|e| e.to_string())?
    };

    if let Ok(handle) = window.hwnd() {
        win32_shell::exclude_from_alt_tab(handle.0 as HWND);
    }

    let monitors = collect_monitors(&app);
    let mut pt = POINT { x: 0, y: 0 };
    let cursor_mon = if unsafe { GetCursorPos(&mut pt) } != 0 {
        monitor_at_point(&monitors, pt.x, pt.y)
    } else {
        None
    };
    if let Some(m) = cursor_mon
        .or_else(|| monitors.iter().find(|m| m.primary))
        .or_else(|| monitors.first())
    {
        let scale = m.scale_factor.max(1.0);
        let w = (460.0 * scale).round() as i32;
        let h = (130.0 * scale).round() as i32;
        let x = m.bounds.left + (m.bounds.width() - w) / 2;
        let y = m.bounds.top;
        let _ = window.set_position(PhysicalPosition::new(x, y));
        let _ = window.set_size(PhysicalSize::new(w as u32, h as u32));
        if let Ok(handle) = window.hwnd() {
            unsafe {
                ShowWindow(handle.0 as HWND, SW_SHOWNOACTIVATE);
                SetWindowPos(
                    handle.0 as HWND,
                    HWND_TOPMOST,
                    x,
                    y,
                    w,
                    h,
                    SWP_NOACTIVATE | SWP_SHOWWINDOW,
                );
            }
        }
    }

    let _ = app.emit(
        "island://show",
        serde_json::json!({
            "label": label,
            "title": title,
            "subtitle": subtitle.unwrap_or_default(),
            "icon": icon.unwrap_or_else(|| "notifications".to_string()),
        }),
    );
    let seq = ISLAND_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
    let app_clone = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(3200));
        if ISLAND_SEQ.load(std::sync::atomic::Ordering::Relaxed) == seq {
            let _ = app_clone.emit("island://hide", ());
            std::thread::sleep(Duration::from_millis(280));
            if ISLAND_SEQ.load(std::sync::atomic::Ordering::Relaxed) == seq {
                if let Some(w) = app_clone.get_webview_window(ISLAND_LABEL) {
                    let _ = w.hide();
                }
            }
        }
    });
    Ok(())
}

#[tauri::command]
fn shell_get_keyboard_layout() -> String {
    let hkl = current_foreground_hkl();
    lang_id_to_code((hkl & 0xFFFF) as u16)
}

fn current_foreground_hkl() -> usize {
    #[link(name = "user32")]
    extern "system" {
        fn GetForegroundWindow() -> HWND;
        fn GetWindowThreadProcessId(hwnd: HWND, pid: *mut u32) -> u32;
        fn GetKeyboardLayout(id_thread: u32) -> usize;
    }
    unsafe {
        let fg = GetForegroundWindow();
        let tid = if fg.is_null() {
            0
        } else {
            GetWindowThreadProcessId(fg, std::ptr::null_mut())
        };
        GetKeyboardLayout(tid)
    }
}

fn lang_id_to_code(lang_id: u16) -> String {
    let primary = lang_id & 0x03FF;
    match primary {
        0x16 => "PT".to_string(),
        0x09 => "US".to_string(),
        0x0C => "FR".to_string(),
        0x0A => "ES".to_string(),
        0x07 => "DE".to_string(),
        0x10 => "IT".to_string(),
        0x11 => "JP".to_string(),
        0x12 => "KO".to_string(),
        0x19 => "RU".to_string(),
        _ => "US".to_string(),
    }
}

fn lang_id_to_name(lang_id: u16) -> (&'static str, &'static str) {
    match lang_id {
        0x0416 => ("Portuguese (Brazil)", "pt-BR"),
        0x0816 => ("Portuguese (Portugal)", "pt-PT"),
        0x0409 => ("English (United States)", "en-US"),
        0x0809 => ("English (United Kingdom)", "en-GB"),
        0x040C => ("French (France)", "fr-FR"),
        0x040A => ("Spanish (Spain)", "es-ES"),
        0x0407 => ("German (Germany)", "de-DE"),
        0x0410 => ("Italian (Italy)", "it-IT"),
        0x0411 => ("Japanese", "ja-JP"),
        _ => match lang_id & 0x03FF {
            0x16 => ("Portuguese", "pt"),
            0x09 => ("English", "en"),
            0x0C => ("French", "fr"),
            0x0A => ("Spanish", "es"),
            0x07 => ("German", "de"),
            _ => ("Keyboard layout", "en"),
        },
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct KeyboardLayoutItem {
    id: String,
    code: String,
    name: String,
    language: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct KeyboardLayoutsPayload {
    current: String,
    items: Vec<KeyboardLayoutItem>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct NetworkInfoPayload {
    connected: bool,
    adapter: String,
    adapter_type: String,
    ipv4: String,
    mac: String,
    link_speed_mbps: u64,
    rx_bytes_per_sec: f64,
    tx_bytes_per_sec: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct NotificationItem {
    id: String,
    app_id: String,
    title: String,
    body: String,
    time: i64,
}

#[tauri::command]
async fn popout_show(
    app: AppHandle,
    monitor_id: Option<String>,
    kind: String,
    anchor_y: f64,
) -> Result<(), String> {
    if win32_shell::is_session_locked() {
        return Ok(());
    }
    if !matches!(kind.as_str(), "audio" | "network" | "keyboard" | "notifications") {
        return Err("invalid-popout-kind".to_string());
    }

    let window = if let Some(existing) = app.get_webview_window(POPOUT_LABEL) {
        existing
    } else {
        WebviewWindowBuilder::new(&app, POPOUT_LABEL, WebviewUrl::App("popout.html".into()))
            .title("Stellaria Popout")
            .decorations(false)
            .shadow(false)
            .transparent(true)
            .resizable(false)
            .skip_taskbar(true)
            .always_on_top(true)
            .visible(false)
            .focused(false)
            .build()
            .map_err(|e| e.to_string())?
    };

    if let Ok(handle) = window.hwnd() {
        win32_shell::exclude_from_alt_tab(handle.0 as HWND);
    }

    let monitors = collect_monitors(&app);
    if let Some(m) = monitor_id
        .as_deref()
        .and_then(|id| monitors.iter().find(|m| m.id == id))
        .or_else(|| monitors.iter().find(|m| m.primary))
        .or_else(|| monitors.first())
    {
        let scale = m.scale_factor.max(1.0);
        let (logical_w, logical_h) = match kind.as_str() {
            "audio" => (396.0, 440.0),
            "network" => (376.0, 320.0),
            "keyboard" => (356.0, 310.0),
            "notifications" => (396.0, 380.0),
            _ => (380.0, 360.0),
        };
        let w = (logical_w * scale).round() as i32;
        let h = (logical_h * scale).round() as i32;
        let bar_w = (52.0 * scale).round() as i32;
        let x = (m.bounds.left + bar_w).clamp(m.bounds.left, (m.bounds.right - w).max(m.bounds.left));
        let anchor_phys_y = m.bounds.top + (anchor_y * scale).round() as i32;
        let desired_y = anchor_phys_y - h + (28.0 * scale).round() as i32;
        let y = desired_y.clamp(m.bounds.top + 8, (m.bounds.bottom - h - 8).max(m.bounds.top));
        let _ = window.set_position(PhysicalPosition::new(x, y));
        let _ = window.set_size(PhysicalSize::new(w as u32, h as u32));
        if let Ok(handle) = window.hwnd() {
            unsafe {
                SetWindowPos(handle.0 as HWND, HWND_TOPMOST, x, y, w, h, SWP_SHOWWINDOW);
            }
        }
    }

    let _ = window.show();
    let _ = window.set_focus();
    let _ = app.emit("popout://show", serde_json::json!({ "kind": kind }));
    Ok(())
}

#[tauri::command]
fn popout_hide(app: AppHandle) {
    if let Some(w) = app.get_webview_window(POPOUT_LABEL) {
        let _ = app.emit("popout://hide", ());
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(240));
            let _ = w.hide();
        });
    }
}

#[tauri::command]
async fn popout_get_audio_sessions() -> Result<Vec<helper_bridge::AudioSessionInfo>, String> {
    helper_bridge::fetch_audio_sessions().await
}

#[tauri::command]
async fn popout_set_audio_session_volume(id: String, volume: u32) -> Result<(), String> {
    helper_bridge::set_audio_session_volume(id, volume).await
}

#[tauri::command]
async fn popout_set_audio_session_mute(id: String, muted: bool) -> Result<(), String> {
    helper_bridge::set_audio_session_mute(id, muted).await
}

#[tauri::command]
async fn popout_get_keyboard_layouts() -> Result<KeyboardLayoutsPayload, String> {
    let current_hkl = current_foreground_hkl();
    let current_id = format!("{:08X}", current_hkl & 0xFFFF_FFFF);

    let topic = helper_bridge::fetch_cached_topic("keyboard").await;
    if let Some(arr) = topic["layouts"].as_array() {
        if !arr.is_empty() {
            let mut items = Vec::with_capacity(arr.len());
            for item in arr {
                let Some(id) = item["id"].as_str() else { continue };
                let lang = item["languageTag"].as_str().unwrap_or("");
                let name = item["name"].as_str().unwrap_or(lang);
                let code = lang
                    .split('-')
                    .last()
                    .unwrap_or(lang)
                    .chars()
                    .take(2)
                    .collect::<String>()
                    .to_uppercase();
                items.push(KeyboardLayoutItem {
                    id: id.to_string(),
                    code,
                    name: name.to_string(),
                    language: lang.to_string(),
                });
            }
            let current = topic["layout"]
                .as_str()
                .map(str::to_string)
                .unwrap_or(current_id);
            return Ok(KeyboardLayoutsPayload { current, items });
        }
    }

    #[link(name = "user32")]
    extern "system" {
        fn GetKeyboardLayoutList(n_buff: i32, lp_list: *mut usize) -> i32;
    }
    let mut hkls = [0usize; 32];
    let count = unsafe { GetKeyboardLayoutList(hkls.len() as i32, hkls.as_mut_ptr()) }.max(0) as usize;
    let mut items = Vec::with_capacity(count);
    for &hkl in &hkls[..count.min(32)] {
        let lang_id = (hkl & 0xFFFF) as u16;
        let (name, language) = lang_id_to_name(lang_id);
        items.push(KeyboardLayoutItem {
            id: format!("{:08X}", hkl & 0xFFFF_FFFF),
            code: lang_id_to_code(lang_id),
            name: name.to_string(),
            language: language.to_string(),
        });
    }
    Ok(KeyboardLayoutsPayload {
        current: current_id,
        items,
    })
}

#[tauri::command]
async fn popout_set_keyboard_layout(id: String) -> Result<(), String> {
    if helper_bridge::set_keyboard_layout_helper(id.clone()).await.is_ok() {
        return Ok(());
    }
    let hkl = usize::from_str_radix(id.trim_start_matches("0x"), 16).map_err(|_| "invalid-hkl".to_string())?;
    #[link(name = "user32")]
    extern "system" {
        fn PostMessageW(hwnd: HWND, msg: u32, wparam: usize, lparam: isize) -> i32;
    }
    const HWND_BROADCAST: HWND = 0xFFFF as HWND;
    const WM_INPUTLANGCHANGEREQUEST: u32 = 0x0050;
    unsafe {
        PostMessageW(HWND_BROADCAST, WM_INPUTLANGCHANGEREQUEST, 0, hkl as isize);
    }
    Ok(())
}

#[tauri::command]
fn popout_get_network_info(state: State<'_, ShellAppState>) -> Result<NetworkInfoPayload, String> {
    let sample = state
        .sampler
        .lock()
        .map_err(|_| "busy".to_string())?
        .sample(None);

    let info = win32_net::query_active_adapter();

    Ok(NetworkInfoPayload {
        connected: info.connected,
        adapter: info.adapter,
        adapter_type: info.adapter_type.to_string(),
        ipv4: info.ipv4,
        mac: info.mac,
        link_speed_mbps: info.link_speed_mbps,
        rx_bytes_per_sec: sample.network.rx_bytes_per_sec as f64,
        tx_bytes_per_sec: sample.network.tx_bytes_per_sec as f64,
    })
}

#[tauri::command]
async fn popout_get_notifications() -> Result<Vec<NotificationItem>, String> {
    let topic = helper_bridge::fetch_cached_topic("notifications").await;
    let arr = topic["items"].as_array().cloned().unwrap_or_default();
    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        let id = item["id"]
            .as_i64()
            .map(|n| n.to_string())
            .or_else(|| item["id"].as_str().map(str::to_string))
            .unwrap_or_default();
        if id.is_empty() {
            continue;
        }
        out.push(NotificationItem {
            id,
            app_id: item["appId"].as_str().unwrap_or("").to_string(),
            title: item["title"].as_str().unwrap_or("").to_string(),
            body: item["body"].as_str().unwrap_or("").to_string(),
            time: item["time"].as_i64().unwrap_or(0),
        });
    }
    Ok(out)
}

#[tauri::command]
fn shell_session_action(app: AppHandle, action: String) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    match action.as_str() {
        "lock" => {
            #[link(name = "user32")]
            extern "system" {
                fn LockWorkStation() -> i32;
            }
            unsafe {
                LockWorkStation();
            }
            Ok(())
        }
        "sleep" => {
            std::process::Command::new("rundll32.exe")
                .args(["powrprof.dll,SetSuspendState", "0,1,0"])
                .creation_flags(CREATE_NO_WINDOW)
                .spawn()
                .map(|_| ())
                .map_err(|e| e.to_string())
        }
        "logout" => {
            win32_shell::restore_windows_shell();
            std::process::Command::new("shutdown.exe")
                .args(["/l"])
                .creation_flags(CREATE_NO_WINDOW)
                .spawn()
                .map(|_| ())
                .map_err(|e| e.to_string())
        }
        "restart" => {
            win32_shell::restore_windows_shell();
            std::process::Command::new("shutdown.exe")
                .args(["/r", "/t", "0"])
                .creation_flags(CREATE_NO_WINDOW)
                .spawn()
                .map(|_| ())
                .map_err(|e| e.to_string())
        }
        "shutdown" => {
            win32_shell::restore_windows_shell();
            std::process::Command::new("shutdown.exe")
                .args(["/s", "/t", "0"])
                .creation_flags(CREATE_NO_WINDOW)
                .spawn()
                .map(|_| ())
                .map_err(|e| e.to_string())
        }
        "restoreShell" => {
            if let Ok(mut hidden) = app.state::<ShellAppState>().windows_taskbar_hidden.lock() {
                *hidden = false;
            }
            win32_shell::restore_windows_shell();
            Ok(())
        }
        "quit" => {
            win32_shell::restore_windows_shell();
            app.exit(0);
            Ok(())
        }
        _ => Err("unknown-session-action".to_string()),
    }
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
                helper_bridge::stop_helper_if_spawned();
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
            helper_bridge::ensure_helper_running();
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
            dashboard_toggle,
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
            launcher_hide,
            preview_show,
            preview_hide,
            popout_show,
            popout_hide,
            popout_get_audio_sessions,
            popout_set_audio_session_volume,
            popout_set_audio_session_mute,
            popout_get_keyboard_layouts,
            popout_set_keyboard_layout,
            popout_get_network_info,
            popout_get_notifications,
            session_toggle,
            session_hide,
            island_show_toast,
            shell_open_settings,
            shell_get_master_audio,
            shell_set_master_volume,
            shell_volume_action,
            shell_get_keyboard_layout,
            shell_set_keep_awake,
            shell_session_action
        ])
        .run(tauri::generate_context!())
        .expect("Stellaria Shell could not start");
}
