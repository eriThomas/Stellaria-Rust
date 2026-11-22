use serde::{Deserialize, Serialize};

use crate::monitors::{MonitorInfo, PhysicalRect};
use crate::shell_state::BarEdge;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FullscreenPolicy {
    HideBar,
    Overlay,
}

/// Computes the deterministic, idempotent work area for `monitor_bounds` when reserving `thickness_px`
/// along `edge`. Calling this with `monitor.bounds` always produces the same result (never shrinks twice).
pub fn compute_reserved_work_area(
    monitor_bounds: PhysicalRect,
    edge: BarEdge,
    thickness_px: i32,
) -> PhysicalRect {
    let thickness = thickness_px.max(0);
    match edge {
        BarEdge::Left => PhysicalRect {
            left: (monitor_bounds.left + thickness).min(monitor_bounds.right - 1),
            ..monitor_bounds
        },
        BarEdge::Right => PhysicalRect {
            right: (monitor_bounds.right - thickness).max(monitor_bounds.left + 1),
            ..monitor_bounds
        },
        BarEdge::Top => PhysicalRect {
            top: (monitor_bounds.top + thickness).min(monitor_bounds.bottom - 1),
            ..monitor_bounds
        },
        BarEdge::Bottom => PhysicalRect {
            bottom: (monitor_bounds.bottom - thickness).max(monitor_bounds.top + 1),
            ..monitor_bounds
        },
    }
}

/// Decides whether a bar on `monitor` should hide for a fullscreen foreground executable.
/// Configurable exceptions (e.g. `wfica32.exe` for Citrix) always force hiding even under `Overlay`.
pub fn should_hide_bar_for_fullscreen(
    monitor: &MonitorInfo,
    policy: FullscreenPolicy,
    foreground_exe: Option<&str>,
    always_hide_exes: &[String],
) -> bool {
    if !monitor.fullscreen {
        return false;
    }
    if policy == FullscreenPolicy::HideBar {
        return true;
    }
    if let Some(exe) = foreground_exe {
        let lower = exe.to_ascii_lowercase();
        if always_hide_exes.iter().any(|rule| lower.contains(&rule.to_ascii_lowercase())) {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reserves_work_area_deterministically_with_negative_coordinates() {
        let arzopa = PhysicalRect {
            left: -1920,
            top: 540,
            right: 0,
            bottom: 1620,
        };
        let left_once = compute_reserved_work_area(arzopa, BarEdge::Left, 60);
        let left_twice = compute_reserved_work_area(arzopa, BarEdge::Left, 60);
        assert_eq!(left_once, left_twice);
        assert_eq!(left_once.left, -1860);
        assert_eq!(left_once.right, 0);

        let bottom = compute_reserved_work_area(arzopa, BarEdge::Bottom, 60);
        assert_eq!(bottom.bottom, 1560);
    }

    #[test]
    fn enforces_citrix_non_overlay_exception() {
        let mon = MonitorInfo {
            id: "DISPLAY1".into(),
            name: "DISPLAY1".into(),
            bounds: PhysicalRect { left: 0, top: 0, right: 1920, bottom: 1080 },
            work_area: PhysicalRect { left: 0, top: 0, right: 1920, bottom: 1080 },
            scale_factor: 1.0,
            primary: true,
            fullscreen: true,
        };
        let exceptions = vec!["wfica32.exe".to_string()];
        assert!(!should_hide_bar_for_fullscreen(&mon, FullscreenPolicy::Overlay, Some("vivaldi.exe"), &exceptions));
        assert!(should_hide_bar_for_fullscreen(&mon, FullscreenPolicy::Overlay, Some("wfica32.exe"), &exceptions));
        assert!(should_hide_bar_for_fullscreen(&mon, FullscreenPolicy::HideBar, Some("vivaldi.exe"), &exceptions));
    }
}
