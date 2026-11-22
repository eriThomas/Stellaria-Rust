use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PhysicalRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl PhysicalRect {
    pub fn width(&self) -> i32 {
        (self.right - self.left).max(0)
    }

    pub fn height(&self) -> i32 {
        (self.bottom - self.top).max(0)
    }

    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.left && x < self.right && y >= self.top && y < self.bottom
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorInfo {
    pub id: String,
    pub name: String,
    pub bounds: PhysicalRect,
    pub work_area: PhysicalRect,
    pub scale_factor: f64,
    pub primary: bool,
    pub fullscreen: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardPlacement {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// Computes the physical-pixel placement for a top-centered dashboard window on `monitor`.
/// Handles negative monitor coordinates, mixed DPI scaling, and clamps strictly to monitor bounds.
pub fn compute_top_dashboard_placement(
    monitor: &MonitorInfo,
    logical_width: f64,
    logical_height: f64,
) -> DashboardPlacement {
    let scale = if monitor.scale_factor.is_finite() && monitor.scale_factor >= 0.5 {
        monitor.scale_factor
    } else {
        1.0
    };
    let max_width = monitor.bounds.width().max(1);
    let max_height = monitor.bounds.height().max(1);

    let desired_width = (logical_width.max(1.0) * scale).round() as i32;
    let desired_height = (logical_height.max(1.0) * scale).round() as i32;

    let width = desired_width.clamp(1, max_width);
    let height = desired_height.clamp(1, max_height);

    let centered_x = monitor.bounds.left + (max_width - width) / 2;
    let x = centered_x.clamp(monitor.bounds.left, monitor.bounds.right - width);
    let y = monitor.bounds.top;

    DashboardPlacement {
        x,
        y,
        width: width as u32,
        height: height as u32,
    }
}

/// Finds the monitor containing the physical cursor coordinates, falling back to primary or first.
pub fn monitor_at_point<'a>(monitors: &'a [MonitorInfo], x: i32, y: i32) -> Option<&'a MonitorInfo> {
    monitors
        .iter()
        .find(|m| m.bounds.contains(x, y))
        .or_else(|| monitors.iter().find(|m| m.primary))
        .or_else(|| monitors.first())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn monitor(id: &str, left: i32, top: i32, width: i32, height: i32, scale: f64, primary: bool) -> MonitorInfo {
        let bounds = PhysicalRect {
            left,
            top,
            right: left + width,
            bottom: top + height,
        };
        MonitorInfo {
            id: id.to_string(),
            name: id.to_string(),
            bounds,
            work_area: bounds,
            scale_factor: scale,
            primary,
            fullscreen: false,
        }
    }

    #[test]
    fn centers_dashboard_on_primary_1080p_monitor() {
        let mon = monitor("DISPLAY1", 0, 0, 1920, 1080, 1.0, true);
        let placement = compute_top_dashboard_placement(&mon, 960.0, 520.0);
        assert_eq!(
            placement,
            DashboardPlacement {
                x: 480,
                y: 0,
                width: 960,
                height: 520,
            }
        );
    }

    #[test]
    fn handles_negative_coordinates_and_mixed_dpi_four_monitor_topology() {
        // Lower-left portable monitor (e.g. Arzopa) at negative X, positive Y, 125% DPI
        let arzopa = monitor("DISPLAY4", -1920, 540, 1920, 1080, 1.25, false);
        let placement = compute_top_dashboard_placement(&arzopa, 900.0, 480.0);
        assert_eq!(placement.width, 1125);
        assert_eq!(placement.height, 600);
        assert_eq!(placement.x, -1920 + (1920 - 1125) / 2);
        assert_eq!(placement.y, 540);
        assert!(placement.x >= arzopa.bounds.left);
        assert!(placement.x + placement.width as i32 <= arzopa.bounds.right);
    }

    #[test]
    fn clamps_oversized_dashboard_to_monitor_bounds() {
        let small = monitor("DISPLAY2", -1280, -720, 1280, 720, 1.5, false);
        let placement = compute_top_dashboard_placement(&small, 1200.0, 800.0);
        assert_eq!(placement.width, 1280);
        assert_eq!(placement.height, 720);
        assert_eq!(placement.x, -1280);
        assert_eq!(placement.y, -720);
    }

    #[test]
    fn resolves_monitor_at_negative_cursor_coordinates() {
        let monitors = vec![
            monitor("PRIMARY", 0, 0, 2560, 1440, 1.0, true),
            monitor("LEFT", -1920, 360, 1920, 1080, 1.25, false),
        ];
        assert_eq!(monitor_at_point(&monitors, -500, 500).map(|m| m.id.as_str()), Some("LEFT"));
        assert_eq!(monitor_at_point(&monitors, 100, 100).map(|m| m.id.as_str()), Some("PRIMARY"));
        assert_eq!(monitor_at_point(&monitors, 99999, 99999).map(|m| m.id.as_str()), Some("PRIMARY"));
    }
}
