use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use stellaria_core::monitors::{MonitorInfo, PhysicalRect};
use stellaria_core::shell_state::{
    FocusSummary, MonitorOverlayState, RecoveryReason, ShellSnapshot, ShellState, TaskSummary,
};
use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, Runtime, State, WebviewUrl,
    WebviewWindowBuilder,
};
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    SetWindowPos, ShowWindow, HWND_TOPMOST, SWP_NOACTIVATE, SWP_SHOWWINDOW, SW_SHOWNOACTIVATE,
};

use crate::dashboard::{exclude_from_alt_tab, DashboardHostState};

pub struct ShellHostState {
    pub state: Mutex<ShellState>,
}

impl ShellHostState {
    pub fn initialize() -> Self {
        let session_generation = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(1);
        let synthetic_bars = std::env::var("STELLARIA_SYNTHETIC_BARS")
            .map(|v| matches!(v.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
            .unwrap_or(false);
        Self {
            state: Mutex::new(ShellState::new(session_generation, synthetic_bars)),
        }
    }
}

pub fn collect_monitors<R: Runtime>(app: &AppHandle<R>) -> Vec<MonitorInfo> {
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

fn destroy_all_synthetic_bars<R: Runtime>(app: &AppHandle<R>) {
    for (label, window) in app.webview_windows() {
        if label.starts_with("bar-") {
            let _ = window.destroy();
        }
    }
}

pub fn sync_synthetic_bar_windows<R: Runtime>(app: &AppHandle<R>, snapshot: &ShellSnapshot) {
    if crate::session_watcher::is_session_locked() {
        return;
    }
    if !snapshot.synthetic_bars_enabled {
        destroy_all_synthetic_bars(app);
        return;
    }

    let expected_labels: Vec<&str> = snapshot
        .monitors
        .iter()
        .map(|m| m.window_label.as_str())
        .collect();

    // Remove orphaned bar windows for disconnected monitors.
    for (label, window) in app.webview_windows() {
        if label.starts_with("bar-") && !expected_labels.contains(&label.as_str()) {
            let _ = window.destroy();
        }
    }

    // Create or update one click-through synthetic bar per physical monitor (NO work-area reservation).
    for view in &snapshot.monitors {
        let label = &view.window_label;
        let p = view.bar_placement;
        let window = if let Some(existing) = app.get_webview_window(label) {
            existing
        } else {
            let url = format!("bar.html?monitor={}", label);
            let Ok(created) = WebviewWindowBuilder::new(app, label, WebviewUrl::App(url.into()))
                .title("Stellaria Synthetic Bar")
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
            let _ = created.set_ignore_cursor_events(true);
            created
        };

        if let Ok(handle) = window.hwnd() {
            exclude_from_alt_tab(handle.0 as HWND);
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

fn propagate_monitors_to_dashboard<R: Runtime>(app: &AppHandle<R>, monitors: Vec<MonitorInfo>) {
    if let Some(dash) = app.try_state::<DashboardHostState>() {
        if let Ok(mut core) = dash.core.lock() {
            if let Some(vis_event) = core.update_monitors(monitors) {
                let _ = app.emit("dashboard://visibility", &vis_event);
            }
        }
    }
}

pub fn reconcile_topology<R: Runtime>(app: &AppHandle<R>, reason: Option<RecoveryReason>) {
    let started = Instant::now();
    let monitors = collect_monitors(app);
    let snapshot = {
        let host = app.state::<ShellHostState>();
        let Ok(mut state) = host.state.lock() else {
            return;
        };
        if let Some(r) = reason {
            state.on_session_boundary(r);
        }
        state.reconcile_monitors(monitors);
        let mon_list = state.monitors().to_vec();
        let snap = state.snapshot();
        drop(state);
        propagate_monitors_to_dashboard(app, mon_list);
        snap
    };

    sync_synthetic_bar_windows(app, &snapshot);
    let _ = app.emit("shell://snapshot", &snapshot);

    if let Some(dash) = app.try_state::<DashboardHostState>() {
        dash.logger
            .timing("topology", "reconciled", started.elapsed().as_millis() as u64);
    }
}

pub fn apply_helper_overlays<R: Runtime>(
    app: &AppHandle<R>,
    overlays: HashMap<String, MonitorOverlayState>,
) {
    let snapshot = {
        let host = app.state::<ShellHostState>();
        let Ok(mut state) = host.state.lock() else {
            return;
        };
        if !state.apply_helper_monitors_overlay(overlays) {
            return;
        }
        let mon_list = state.monitors().to_vec();
        let snap = state.snapshot();
        drop(state);
        propagate_monitors_to_dashboard(app, mon_list);
        snap
    };
    sync_synthetic_bar_windows(app, &snapshot);
    let _ = app.emit("shell://snapshot", &snapshot);
}

pub fn apply_helper_tasks<R: Runtime>(app: &AppHandle<R>, tasks: Vec<TaskSummary>) {
    let snapshot = {
        let host = app.state::<ShellHostState>();
        let Ok(mut state) = host.state.lock() else {
            return;
        };
        if !state.apply_helper_tasks(tasks) {
            return;
        }
        state.snapshot()
    };
    let _ = app.emit("shell://snapshot", &snapshot);
}

pub fn apply_helper_focus<R: Runtime>(app: &AppHandle<R>, focus: FocusSummary) {
    let snapshot = {
        let host = app.state::<ShellHostState>();
        let Ok(mut state) = host.state.lock() else {
            return;
        };
        if !state.apply_helper_focus(focus) {
            return;
        }
        state.snapshot()
    };
    let _ = app.emit("shell://snapshot", &snapshot);
}

pub fn set_helper_connected<R: Runtime>(app: &AppHandle<R>, connected: bool) {
    let snapshot = {
        let host = app.state::<ShellHostState>();
        let Ok(mut state) = host.state.lock() else {
            return;
        };
        if !state.set_helper_connected(connected) {
            return;
        }
        state.snapshot()
    };
    let _ = app.emit("shell://snapshot", &snapshot);
}

pub fn toggle_synthetic_bars<R: Runtime>(app: &AppHandle<R>) {
    let snapshot = {
        let host = app.state::<ShellHostState>();
        let Ok(mut state) = host.state.lock() else {
            return;
        };
        let next = !state.synthetic_bars_enabled();
        state.set_synthetic_bars_enabled(next);
        if next && state.monitors().is_empty() {
            state.reconcile_monitors(collect_monitors(app));
        }
        state.snapshot()
    };
    sync_synthetic_bar_windows(app, &snapshot);
    let _ = app.emit("shell://snapshot", &snapshot);
}

#[tauri::command]
pub fn shell_get_snapshot(app: AppHandle, state: State<'_, ShellHostState>) -> Result<ShellSnapshot, String> {
    let mut core = state.state.lock().map_err(|_| "busy".to_string())?;
    if core.monitors().is_empty() {
        core.reconcile_monitors(collect_monitors(&app));
    }
    Ok(core.snapshot())
}

#[tauri::command]
pub fn shell_set_synthetic_bars_enabled(
    app: AppHandle,
    state: State<'_, ShellHostState>,
    enabled: bool,
) -> Result<ShellSnapshot, String> {
    let snapshot = {
        let mut core = state.state.lock().map_err(|_| "busy".to_string())?;
        core.set_synthetic_bars_enabled(enabled);
        if enabled && core.monitors().is_empty() {
            core.reconcile_monitors(collect_monitors(&app));
        }
        core.snapshot()
    };
    sync_synthetic_bar_windows(&app, &snapshot);
    let _ = app.emit("shell://snapshot", &snapshot);
    Ok(snapshot)
}
