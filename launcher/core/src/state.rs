use serde::{Deserialize, Serialize};

use crate::flags::FeatureFlags;
use crate::monitors::MonitorInfo;
use crate::settings::{DashboardSettings, DashboardSettingsPatch};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DashboardTab {
    Dash,
    Media,
    Performance,
    Weather,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OpenTrigger {
    Click,
    Hotkey,
    Tray,
    Hover,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VisibilityState {
    pub open: bool,
    pub monitor_id: Option<String>,
    pub trigger: Option<OpenTrigger>,
    pub suppressed: bool,
    pub active_tab: DashboardTab,
}

impl Default for VisibilityState {
    fn default() -> Self {
        Self {
            open: false,
            monitor_id: None,
            trigger: None,
            suppressed: false,
            active_tab: DashboardTab::Dash,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VisibilityEvent {
    pub revision: u64,
    pub visibility: VisibilityState,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardSnapshot {
    pub revision: u64,
    pub session_generation: u64,
    pub visibility: VisibilityState,
    pub settings: DashboardSettings,
    pub monitors: Vec<MonitorInfo>,
    pub flags: FeatureFlags,
}

pub struct DashboardCore {
    revision: u64,
    session_generation: u64,
    visibility: VisibilityState,
    settings: DashboardSettings,
    monitors: Vec<MonitorInfo>,
    flags: FeatureFlags,
}

impl DashboardCore {
    pub fn new(session_generation: u64, settings: DashboardSettings, flags: FeatureFlags) -> Self {
        Self {
            revision: 1,
            session_generation,
            visibility: VisibilityState::default(),
            settings,
            monitors: Vec::new(),
            flags,
        }
    }

    pub fn snapshot(&self) -> DashboardSnapshot {
        DashboardSnapshot {
            revision: self.revision,
            session_generation: self.session_generation,
            visibility: self.visibility.clone(),
            settings: self.settings.clone(),
            monitors: self.monitors.clone(),
            flags: self.flags.clone(),
        }
    }

    pub fn flags(&self) -> &FeatureFlags {
        &self.flags
    }

    pub fn settings(&self) -> &DashboardSettings {
        &self.settings
    }

    pub fn visibility(&self) -> &VisibilityState {
        &self.visibility
    }

    pub fn should_poll_telemetry(&self) -> bool {
        self.visibility.open && !self.visibility.suppressed
    }

    pub fn should_poll_gpu(&self) -> bool {
        self.should_poll_telemetry() && self.visibility.active_tab == DashboardTab::Performance
    }

    pub fn set_active_tab(&mut self, tab: DashboardTab) -> VisibilityEvent {
        if self.visibility.active_tab != tab {
            self.visibility.active_tab = tab;
            self.revision += 1;
        }
        VisibilityEvent {
            revision: self.revision,
            visibility: self.visibility.clone(),
        }
    }

    pub fn set_visibility(
        &mut self,
        open: bool,
        monitor_id: Option<String>,
        trigger: OpenTrigger,
    ) -> Result<VisibilityEvent, String> {
        if !open {
            if self.visibility.open || self.visibility.trigger.is_some() {
                self.visibility.open = false;
                self.visibility.trigger = None;
                self.revision += 1;
            }
            return Ok(VisibilityEvent {
                revision: self.revision,
                visibility: self.visibility.clone(),
            });
        }

        let target = if let Some(requested) = monitor_id {
            let found = self.monitors.iter().find(|m| m.id == requested);
            if !self.monitors.is_empty() && found.is_none() {
                return Err("unknown-monitor".to_string());
            }
            found.cloned()
        } else {
            self.monitors
                .iter()
                .find(|m| m.primary)
                .or_else(|| self.monitors.first())
                .cloned()
        };

        let suppressed = target.as_ref().is_some_and(|m| m.fullscreen);
        self.visibility.monitor_id = target.map(|m| m.id);
        self.visibility.suppressed = suppressed;
        self.visibility.open = !suppressed;
        self.visibility.trigger = if suppressed { None } else { Some(trigger) };
        self.revision += 1;

        Ok(VisibilityEvent {
            revision: self.revision,
            visibility: self.visibility.clone(),
        })
    }

    pub fn update_monitors(&mut self, monitors: Vec<MonitorInfo>) -> Option<VisibilityEvent> {
        self.monitors = monitors;
        self.revision += 1;

        if let Some(active_id) = &self.visibility.monitor_id {
            let active = self.monitors.iter().find(|m| &m.id == active_id);
            match active {
                Some(mon) if mon.fullscreen => {
                    self.visibility.suppressed = true;
                    self.visibility.open = false;
                    self.visibility.trigger = None;
                    return Some(VisibilityEvent {
                        revision: self.revision,
                        visibility: self.visibility.clone(),
                    });
                }
                None if self.visibility.open => {
                    // Active monitor was disconnected: close cleanly instead of leaving an orphaned window.
                    self.visibility.open = false;
                    self.visibility.trigger = None;
                    self.visibility.monitor_id = None;
                    return Some(VisibilityEvent {
                        revision: self.revision,
                        visibility: self.visibility.clone(),
                    });
                }
                Some(mon) => {
                    self.visibility.suppressed = mon.fullscreen;
                }
                None => {}
            }
        }
        None
    }

    pub fn update_settings(&mut self, patch: DashboardSettingsPatch) -> Result<(u64, DashboardSettings), String> {
        self.settings.apply_patch(patch)?;
        self.revision += 1;
        Ok((self.revision, self.settings.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::monitors::PhysicalRect;

    fn sample_monitors(primary_fullscreen: bool) -> Vec<MonitorInfo> {
        let bounds = PhysicalRect { left: 0, top: 0, right: 1920, bottom: 1080 };
        vec![
            MonitorInfo {
                id: "DISPLAY1".into(),
                name: "Primary".into(),
                bounds,
                work_area: bounds,
                scale_factor: 1.0,
                primary: true,
                fullscreen: primary_fullscreen,
            },
            MonitorInfo {
                id: "DISPLAY2".into(),
                name: "Secondary".into(),
                bounds: PhysicalRect { left: -1920, top: 0, right: 0, bottom: 1080 },
                work_area: PhysicalRect { left: -1920, top: 0, right: 0, bottom: 1080 },
                scale_factor: 1.25,
                primary: false,
                fullscreen: false,
            },
        ]
    }

    #[test]
    fn hidden_dashboard_never_polls_telemetry_or_gpu() {
        let mut core = DashboardCore::new(1, DashboardSettings::default(), FeatureFlags::experimental());
        core.update_monitors(sample_monitors(false));

        assert!(!core.should_poll_telemetry());
        assert!(!core.should_poll_gpu());

        core.set_visibility(true, Some("DISPLAY1".into()), OpenTrigger::Tray).unwrap();
        assert!(core.should_poll_telemetry());
        assert!(!core.should_poll_gpu());

        core.set_active_tab(DashboardTab::Performance);
        assert!(core.should_poll_gpu());

        core.set_visibility(false, None, OpenTrigger::Tray).unwrap();
        assert!(!core.should_poll_telemetry());
        assert!(!core.should_poll_gpu());
    }

    #[test]
    fn fullscreen_transition_suppresses_and_closes_active_dashboard() {
        let mut core = DashboardCore::new(1, DashboardSettings::default(), FeatureFlags::experimental());
        core.update_monitors(sample_monitors(false));
        let opened = core.set_visibility(true, Some("DISPLAY1".into()), OpenTrigger::Click).unwrap();
        assert!(opened.visibility.open);

        let suppressed = core.update_monitors(sample_monitors(true)).expect("visibility event emitted");
        assert!(!suppressed.visibility.open);
        assert!(suppressed.visibility.suppressed);
        assert!(suppressed.revision > opened.revision);
        assert!(!core.should_poll_telemetry());
    }

    #[test]
    fn rejects_unknown_monitor_id() {
        let mut core = DashboardCore::new(1, DashboardSettings::default(), FeatureFlags::experimental());
        core.update_monitors(sample_monitors(false));
        let result = core.set_visibility(true, Some("DISPLAY999".into()), OpenTrigger::Click);
        assert_eq!(result, Err("unknown-monitor".into()));
    }

    #[test]
    fn disconnecting_active_monitor_closes_dashboard() {
        let mut core = DashboardCore::new(1, DashboardSettings::default(), FeatureFlags::experimental());
        core.update_monitors(sample_monitors(false));
        core.set_visibility(true, Some("DISPLAY2".into()), OpenTrigger::Click).unwrap();

        let remaining = vec![sample_monitors(false).remove(0)];
        let event = core.update_monitors(remaining).expect("disconnect closes dashboard");
        assert!(!event.visibility.open);
        assert_eq!(event.visibility.monitor_id, None);
    }
}
