use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiskTelemetry {
    pub mount_point: String,
    pub total_bytes: u64,
    pub available_bytes: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkTelemetry {
    pub rx_bytes_per_sec: u64,
    pub tx_bytes_per_sec: u64,
    pub total_rx_bytes: u64,
    pub total_tx_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GpuTelemetry {
    pub name: String,
    pub usage_percent: Option<f64>,
    pub temperature_c: Option<f64>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TelemetrySnapshot {
    pub revision: u64,
    pub uptime_ms: u64,
    pub cpu_percent: f64,
    pub memory_used_bytes: u64,
    pub memory_total_bytes: u64,
    pub disks: Vec<DiskTelemetry>,
    pub network: NetworkTelemetry,
    pub gpu: Option<GpuTelemetry>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemInfo {
    pub os: String,
    pub cpu: String,
    pub gpu: Option<String>,
    pub user: String,
    pub picture_base64: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaSessionInfo {
    pub session_id: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub is_playing: bool,
    pub position_secs: f64,
    pub duration_secs: f64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaSnapshot {
    pub current: Option<MediaSessionInfo>,
    pub sessions: Vec<MediaSessionInfo>,
    pub artwork_base64: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MediaAction {
    TogglePlayPause,
    Next,
    Previous,
}

impl MediaAction {
    pub fn as_helper_action(self) -> &'static str {
        match self {
            MediaAction::TogglePlayPause => "playPause",
            MediaAction::Next => "next",
            MediaAction::Previous => "previous",
        }
    }
}

/// Computes CPU utilization percentage (0.0..=100.0) from two consecutive Win32 `GetSystemTimes` readings.
pub fn compute_cpu_percent(
    prev_idle: u64,
    prev_kernel: u64,
    prev_user: u64,
    curr_idle: u64,
    curr_kernel: u64,
    curr_user: u64,
) -> f64 {
    let idle = curr_idle.saturating_sub(prev_idle);
    let kernel = curr_kernel.saturating_sub(prev_kernel);
    let user = curr_user.saturating_sub(prev_user);
    let total = kernel.saturating_add(user);
    if total == 0 {
        return 0.0;
    }
    let busy = total.saturating_sub(idle);
    ((busy as f64 / total as f64) * 100.0).clamp(0.0, 100.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computes_cpu_percent_accurately_and_handles_zero_delta() {
        assert_eq!(compute_cpu_percent(100, 200, 100, 100, 200, 100), 0.0);
        // kernel delta = 100 (includes 75 idle), user delta = 100 -> total 200, busy 125 -> 62.5%
        let pct = compute_cpu_percent(100, 200, 100, 175, 300, 200);
        assert!((pct - 62.5).abs() < 0.001);
    }

    #[test]
    fn maps_media_actions_to_helper_verbs() {
        assert_eq!(MediaAction::TogglePlayPause.as_helper_action(), "playPause");
        assert_eq!(MediaAction::Next.as_helper_action(), "next");
        assert_eq!(MediaAction::Previous.as_helper_action(), "previous");
    }
}
