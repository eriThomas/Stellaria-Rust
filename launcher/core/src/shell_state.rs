use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::monitors::MonitorInfo;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BarEdge {
    Left,
    Right,
    Top,
    Bottom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RecoveryReason {
    Startup,
    LockUnlock,
    SleepResume,
    ExplorerRestart,
    DisplayChanged,
    HelperReconnected,
    HelperDisconnected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskSummary {
    pub id: u64,
    pub title: String,
    pub exe: String,
    pub monitor_id: String,
    pub minimized: bool,
    pub active: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FocusSummary {
    pub hwnd: Option<u64>,
    pub title: String,
    pub exe: String,
    pub monitor_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorOverlayState {
    pub fullscreen: bool,
    pub covered: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BarWindowPlacement {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MonitorReconcileDiff {
    pub added: Vec<String>,
    pub updated: Vec<String>,
    pub removed: Vec<String>,
}

impl MonitorReconcileDiff {
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.updated.is_empty() && self.removed.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShellMonitorView {
    pub monitor: MonitorInfo,
    pub covered: bool,
    pub window_label: String,
    pub bar_placement: BarWindowPlacement,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShellSnapshot {
    pub revision: u64,
    pub session_generation: u64,
    pub topology_revision: u64,
    pub helper_connected: bool,
    pub synthetic_bars_enabled: bool,
    pub bar_edge: BarEdge,
    pub bar_thickness_logical: f64,
    pub monitors: Vec<ShellMonitorView>,
    pub task_count: usize,
    pub tasks: Vec<TaskSummary>,
    pub focus: FocusSummary,
    pub last_recovery: RecoveryReason,
}

/// Converts a Windows monitor device name (e.g. `\\.\DISPLAY1`) into a deterministic, Tauri-safe window label.
pub fn bar_window_label(monitor_id: &str) -> String {
    let mut slug = String::with_capacity(monitor_id.len());
    for ch in monitor_id.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
            slug.push(ch.to_ascii_lowercase());
        }
    }
    if slug.is_empty() {
        slug.push_str("unknown");
    }
    format!("bar-{slug}")
}

/// Computes physical-pixel placement for a per-monitor bar attached to `edge`, clamped strictly to `monitor.bounds`.
pub fn compute_bar_placement(
    monitor: &MonitorInfo,
    edge: BarEdge,
    logical_thickness: f64,
) -> BarWindowPlacement {
    let scale = if monitor.scale_factor.is_finite() && monitor.scale_factor >= 0.5 {
        monitor.scale_factor
    } else {
        1.0
    };
    let mon_w = monitor.bounds.width().max(1);
    let mon_h = monitor.bounds.height().max(1);
    let thickness = (logical_thickness.max(1.0) * scale).round() as i32;

    match edge {
        BarEdge::Left => {
            let w = thickness.clamp(1, mon_w);
            BarWindowPlacement {
                x: monitor.bounds.left,
                y: monitor.bounds.top,
                width: w as u32,
                height: mon_h as u32,
            }
        }
        BarEdge::Right => {
            let w = thickness.clamp(1, mon_w);
            BarWindowPlacement {
                x: monitor.bounds.right - w,
                y: monitor.bounds.top,
                width: w as u32,
                height: mon_h as u32,
            }
        }
        BarEdge::Top => {
            let h = thickness.clamp(1, mon_h);
            BarWindowPlacement {
                x: monitor.bounds.left,
                y: monitor.bounds.top,
                width: mon_w as u32,
                height: h as u32,
            }
        }
        BarEdge::Bottom => {
            let h = thickness.clamp(1, mon_h);
            BarWindowPlacement {
                x: monitor.bounds.left,
                y: monitor.bounds.bottom - h,
                width: mon_w as u32,
                height: h as u32,
            }
        }
    }
}

pub struct ShellState {
    revision: u64,
    session_generation: u64,
    topology_revision: u64,
    helper_connected: bool,
    synthetic_bars_enabled: bool,
    bar_edge: BarEdge,
    bar_thickness_logical: f64,
    monitors: Vec<MonitorInfo>,
    overlays: HashMap<String, MonitorOverlayState>,
    tasks: Vec<TaskSummary>,
    focus: FocusSummary,
    last_recovery: RecoveryReason,
}

impl ShellState {
    pub fn new(session_generation: u64, synthetic_bars_enabled: bool) -> Self {
        Self {
            revision: 1,
            session_generation,
            topology_revision: 1,
            helper_connected: false,
            synthetic_bars_enabled,
            bar_edge: BarEdge::Left,
            bar_thickness_logical: 48.0,
            monitors: Vec::new(),
            overlays: HashMap::new(),
            tasks: Vec::new(),
            focus: FocusSummary::default(),
            last_recovery: RecoveryReason::Startup,
        }
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn session_generation(&self) -> u64 {
        self.session_generation
    }

    pub fn topology_revision(&self) -> u64 {
        self.topology_revision
    }

    pub fn synthetic_bars_enabled(&self) -> bool {
        self.synthetic_bars_enabled
    }

    pub fn set_synthetic_bars_enabled(&mut self, enabled: bool) -> bool {
        if self.synthetic_bars_enabled == enabled {
            return false;
        }
        self.synthetic_bars_enabled = enabled;
        self.revision += 1;
        true
    }

    pub fn set_bar_edge(&mut self, edge: BarEdge) -> bool {
        if self.bar_edge == edge {
            return false;
        }
        self.bar_edge = edge;
        self.topology_revision += 1;
        self.revision += 1;
        true
    }

    pub fn set_helper_connected(&mut self, connected: bool) -> bool {
        if self.helper_connected == connected {
            return false;
        }
        self.helper_connected = connected;
        self.revision += 1;
        if !connected {
            self.tasks.clear();
            self.focus = FocusSummary::default();
            self.last_recovery = RecoveryReason::HelperDisconnected;
        } else {
            self.last_recovery = RecoveryReason::HelperReconnected;
        }
        true
    }

    pub fn on_session_boundary(&mut self, reason: RecoveryReason) {
        self.session_generation += 1;
        self.topology_revision += 1;
        self.revision += 1;
        self.last_recovery = reason;
        // Invalidate transient HWNDs on session boundaries (Lock/Unlock, Sleep/Resume, Explorer restart).
        if matches!(
            reason,
            RecoveryReason::LockUnlock | RecoveryReason::SleepResume | RecoveryReason::ExplorerRestart
        ) {
            self.tasks.clear();
            self.focus = FocusSummary::default();
        }
    }

    pub fn reconcile_monitors(&mut self, incoming: Vec<MonitorInfo>) -> MonitorReconcileDiff {
        let mut merged: Vec<MonitorInfo> = incoming
            .into_iter()
            .map(|mut mon| {
                if let Some(overlay) = self.overlays.get(&mon.id) {
                    mon.fullscreen = overlay.fullscreen;
                }
                mon
            })
            .collect();
        merged.sort_by(|a, b| a.id.cmp(&b.id));

        let prev_by_id: HashMap<&str, &MonitorInfo> =
            self.monitors.iter().map(|m| (m.id.as_str(), m)).collect();
        let next_ids: HashSet<&str> = merged.iter().map(|m| m.id.as_str()).collect();

        let mut diff = MonitorReconcileDiff::default();
        for mon in &merged {
            match prev_by_id.get(mon.id.as_str()) {
                None => diff.added.push(mon.id.clone()),
                Some(old) if *old != mon => diff.updated.push(mon.id.clone()),
                Some(_) => {}
            }
        }
        for old in &self.monitors {
            if !next_ids.contains(old.id.as_str()) {
                diff.removed.push(old.id.clone());
            }
        }

        if !diff.is_empty() {
            self.monitors = merged;
            self.topology_revision += 1;
            self.revision += 1;
        }
        diff
    }

    pub fn apply_helper_monitors_overlay(
        &mut self,
        overlays: HashMap<String, MonitorOverlayState>,
    ) -> bool {
        if self.overlays == overlays {
            return false;
        }
        self.overlays = overlays;
        for mon in &mut self.monitors {
            if let Some(ov) = self.overlays.get(&mon.id) {
                mon.fullscreen = ov.fullscreen;
            }
        }
        self.revision += 1;
        true
    }

    pub fn apply_helper_tasks(&mut self, tasks: Vec<TaskSummary>) -> bool {
        if self.tasks == tasks {
            return false;
        }
        self.tasks = tasks;
        self.revision += 1;
        true
    }

    pub fn apply_helper_focus(&mut self, focus: FocusSummary) -> bool {
        if self.focus == focus {
            return false;
        }
        self.focus = focus;
        self.revision += 1;
        true
    }

    pub fn monitors(&self) -> &[MonitorInfo] {
        &self.monitors
    }

    pub fn snapshot(&self) -> ShellSnapshot {
        let monitors = self
            .monitors
            .iter()
            .map(|mon| {
                let covered = self
                    .overlays
                    .get(&mon.id)
                    .map(|ov| ov.covered)
                    .unwrap_or(mon.fullscreen);
                ShellMonitorView {
                    window_label: bar_window_label(&mon.id),
                    bar_placement: compute_bar_placement(mon, self.bar_edge, self.bar_thickness_logical),
                    monitor: mon.clone(),
                    covered,
                }
            })
            .collect();

        ShellSnapshot {
            revision: self.revision,
            session_generation: self.session_generation,
            topology_revision: self.topology_revision,
            helper_connected: self.helper_connected,
            synthetic_bars_enabled: self.synthetic_bars_enabled,
            bar_edge: self.bar_edge,
            bar_thickness_logical: self.bar_thickness_logical,
            monitors,
            task_count: self.tasks.len(),
            tasks: self.tasks.clone(),
            focus: self.focus.clone(),
            last_recovery: self.last_recovery,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::monitors::PhysicalRect;

    fn mon(id: &str, left: i32, top: i32, w: i32, h: i32, scale: f64, primary: bool) -> MonitorInfo {
        let bounds = PhysicalRect {
            left,
            top,
            right: left + w,
            bottom: top + h,
        };
        MonitorInfo {
            id: id.into(),
            name: id.into(),
            bounds,
            work_area: bounds,
            scale_factor: scale,
            primary,
            fullscreen: false,
        }
    }

    #[test]
    fn computes_bar_placement_across_all_four_edges_with_negative_coords_and_mixed_dpi() {
        let arzopa = mon(r"\\.\DISPLAY4", -1920, 540, 1920, 1080, 1.25, false);

        let left = compute_bar_placement(&arzopa, BarEdge::Left, 48.0);
        assert_eq!(left, BarWindowPlacement { x: -1920, y: 540, width: 60, height: 1080 });

        let right = compute_bar_placement(&arzopa, BarEdge::Right, 48.0);
        assert_eq!(right, BarWindowPlacement { x: -60, y: 540, width: 60, height: 1080 });

        let top = compute_bar_placement(&arzopa, BarEdge::Top, 48.0);
        assert_eq!(top, BarWindowPlacement { x: -1920, y: 540, width: 1920, height: 60 });

        let bottom = compute_bar_placement(&arzopa, BarEdge::Bottom, 48.0);
        assert_eq!(bottom, BarWindowPlacement { x: -1920, y: 1560, width: 1920, height: 60 });
    }

    #[test]
    fn sanitizes_windows_device_ids_into_deterministic_labels() {
        assert_eq!(bar_window_label(r"\\.\DISPLAY1"), "bar-display1");
        assert_eq!(bar_window_label(r"\\.\DISPLAY4"), "bar-display4");
        assert_eq!(bar_window_label("///"), "bar-unknown");
    }

    #[test]
    fn reconciles_monitor_add_update_and_disconnect_idempotently() {
        let mut state = ShellState::new(1, true);
        let m1 = mon(r"\\.\DISPLAY1", 0, 0, 2560, 1440, 1.0, true);
        let m2 = mon(r"\\.\DISPLAY2", -1920, 360, 1920, 1080, 1.25, false);

        let d1 = state.reconcile_monitors(vec![m1.clone(), m2.clone()]);
        assert_eq!(d1.added.len(), 2);
        assert!(d1.removed.is_empty());
        let rev_after_add = state.revision();

        // Identical topology must be a no-op (no revision bump, no window churn).
        let d2 = state.reconcile_monitors(vec![m2.clone(), m1.clone()]);
        assert!(d2.is_empty());
        assert_eq!(state.revision(), rev_after_add);

        // Disconnect DISPLAY2 -> reported in removed.
        let d3 = state.reconcile_monitors(vec![m1]);
        assert_eq!(d3.removed, vec![r"\\.\DISPLAY2".to_string()]);
        assert!(state.revision() > rev_after_add);
    }

    #[test]
    fn session_boundary_increments_generation_and_clears_stale_tasks() {
        let mut state = ShellState::new(1, true);
        state.apply_helper_tasks(vec![TaskSummary {
            id: 42,
            title: "Vivaldi".into(),
            exe: "vivaldi.exe".into(),
            monitor_id: r"\\.\DISPLAY1".into(),
            minimized: false,
            active: true,
        }]);
        assert_eq!(state.snapshot().task_count, 1);

        state.on_session_boundary(RecoveryReason::LockUnlock);
        let snap = state.snapshot();
        assert_eq!(snap.session_generation, 2);
        assert_eq!(snap.task_count, 0);
        assert_eq!(snap.last_recovery, RecoveryReason::LockUnlock);
    }
}
