use std::collections::{HashMap, HashSet};
use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowIdentity {
    pub hwnd: u64,
    pub pid: u32,
    pub session_generation: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskWindow {
    pub hwnd: u64,
    pub pid: u32,
    pub title: String,
    pub exe_path: String,
    pub exe_name: String,
    pub monitor_id: String,
    pub minimized: bool,
    pub active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PinnedApp {
    pub key: String,
    pub exe_path: String,
    pub display_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskGroup {
    pub key: String,
    pub display_name: String,
    pub exe_path: String,
    pub pinned: bool,
    pub active: bool,
    pub window_count: usize,
    pub windows: Vec<TaskWindow>,
}

/// Normalizes an application key case-insensitively: prefers executable filename, then path, then HWND.
pub fn normalize_app_key(exe_name: &str, exe_path: &str, hwnd: u64) -> String {
    let trimmed_name = exe_name.trim();
    if !trimmed_name.is_empty() {
        return trimmed_name.to_ascii_lowercase();
    }
    let trimmed_path = exe_path.trim();
    if !trimmed_path.is_empty() {
        let base = trimmed_path
            .rsplit(['\\', '/'])
            .next()
            .unwrap_or(trimmed_path);
        if !base.is_empty() {
            return base.to_ascii_lowercase();
        }
    }
    format!("hwnd:{hwnd}")
}

pub fn display_name_from_exe(exe_name: &str, fallback: &str) -> String {
    let trimmed = exe_name.trim();
    let stripped = trimmed
        .strip_suffix(".exe")
        .or_else(|| trimmed.strip_suffix(".EXE"))
        .unwrap_or(trimmed);
    if !stripped.is_empty() {
        let mut chars = stripped.chars();
        if let Some(first) = chars.next() {
            return format!("{}{}", first.to_uppercase(), chars.as_str());
        }
    }
    fallback.to_string()
}

/// Builds deterministic taskbar groups:
/// - Pinned apps stay first in pin order, survive closing all windows, and are never duplicated.
/// - Running windows group case-insensitively by normalized executable key when `grouping_enabled` is true.
/// - Optional `filter_monitor` filters windows to a single monitor (pinned apps remain visible).
pub fn build_task_groups(
    windows: &[TaskWindow],
    pins: &[PinnedApp],
    filter_monitor: Option<&str>,
    grouping_enabled: bool,
) -> Vec<TaskGroup> {
    let filtered: Vec<&TaskWindow> = windows
        .iter()
        .filter(|w| match filter_monitor {
            Some(mon) => w.monitor_id.is_empty() || w.monitor_id == mon,
            None => true,
        })
        .collect();

    let mut groups: Vec<TaskGroup> = Vec::new();
    let mut index_by_key: HashMap<String, usize> = HashMap::new();
    let mut seen_pin_keys: HashSet<String> = HashSet::new();

    for pin in pins {
        let key = pin.key.to_ascii_lowercase();
        if !seen_pin_keys.insert(key.clone()) {
            continue;
        }
        index_by_key.insert(key.clone(), groups.len());
        groups.push(TaskGroup {
            key,
            display_name: pin.display_name.clone(),
            exe_path: pin.exe_path.clone(),
            pinned: true,
            active: false,
            window_count: 0,
            windows: Vec::new(),
        });
    }

    for win in filtered {
        let base_key = normalize_app_key(&win.exe_name, &win.exe_path, win.hwnd);
        let group_key = if grouping_enabled {
            base_key.clone()
        } else if index_by_key.contains_key(&base_key)
            && groups[index_by_key[&base_key]].windows.is_empty()
        {
            base_key.clone()
        } else {
            format!("{base_key}#{}", win.hwnd)
        };

        if let Some(&idx) = index_by_key.get(&group_key) {
            let group = &mut groups[idx];
            if group.exe_path.is_empty() && !win.exe_path.is_empty() {
                group.exe_path = win.exe_path.clone();
            }
            group.active |= win.active;
            group.windows.push((*win).clone());
            group.window_count = group.windows.len();
        } else {
            index_by_key.insert(group_key.clone(), groups.len());
            groups.push(TaskGroup {
                key: group_key,
                display_name: display_name_from_exe(&win.exe_name, &win.title),
                exe_path: win.exe_path.clone(),
                pinned: false,
                active: win.active,
                window_count: 1,
                windows: vec![(*win).clone()],
            });
        }
    }

    groups
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PinStore {
    pub schema_version: u32,
    pub pins: Vec<PinnedApp>,
}

impl PinStore {
    pub fn load_or_default(path: &Path) -> Self {
        let Ok(raw) = std::fs::read_to_string(path) else {
            return Self {
                schema_version: 1,
                pins: Vec::new(),
            };
        };
        serde_json::from_str(&raw).unwrap_or(Self {
            schema_version: 1,
            pins: Vec::new(),
        })
    }

    pub fn save_atomic(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        let tmp = path.with_extension(format!("tmp.{}", std::process::id()));
        std::fs::write(&tmp, json).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, path).map_err(|e| {
            let _ = std::fs::remove_file(&tmp);
            e.to_string()
        })
    }

    pub fn toggle_pin(&mut self, exe_path: &str, exe_name: &str, display_name: Option<&str>) -> bool {
        let key = normalize_app_key(exe_name, exe_path, 0);
        if let Some(pos) = self.pins.iter().position(|p| p.key.eq_ignore_ascii_case(&key)) {
            self.pins.remove(pos);
            false
        } else {
            self.pins.push(PinnedApp {
                key,
                exe_path: exe_path.to_string(),
                display_name: display_name
                    .filter(|s| !s.trim().is_empty())
                    .map(str::to_string)
                    .unwrap_or_else(|| display_name_from_exe(exe_name, exe_name)),
            });
            self.schema_version = 1;
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn win(hwnd: u64, exe: &str, title: &str, mon: &str, active: bool) -> TaskWindow {
        TaskWindow {
            hwnd,
            pid: (hwnd * 10) as u32,
            title: title.into(),
            exe_path: format!(r"C:\Apps\{exe}"),
            exe_name: exe.into(),
            monitor_id: mon.into(),
            minimized: false,
            active,
        }
    }

    #[test]
    fn groups_same_executable_case_insensitively_across_four_monitors() {
        let windows = vec![
            win(1, "Vivaldi.exe", "Tab 1", "DISPLAY1", true),
            win(2, "vivaldi.exe", "Tab 2", "DISPLAY2", false),
            win(3, "VIVALDI.EXE", "Tab 3", "DISPLAY3", false),
            win(4, "vivaldi.exe", "Tab 4", "DISPLAY4", false),
            win(5, "Code.exe", "Stellaria", "DISPLAY1", false),
        ];

        let groups = build_task_groups(&windows, &[], None, true);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].key, "vivaldi.exe");
        assert_eq!(groups[0].window_count, 4);
        assert!(groups[0].active);
        assert_eq!(groups[1].key, "code.exe");
        assert_eq!(groups[1].window_count, 1);
    }

    #[test]
    fn pinned_apps_stay_first_survive_closing_and_never_duplicate() {
        let pins = vec![PinnedApp {
            key: "vivaldi.exe".into(),
            exe_path: r"C:\Apps\vivaldi.exe".into(),
            display_name: "Vivaldi".into(),
        }];

        // Running: Vivaldi + Code -> Vivaldi stays first, not duplicated
        let running = vec![
            win(10, "Code.exe", "Editor", "DISPLAY1", true),
            win(11, "vivaldi.exe", "Browser", "DISPLAY2", false),
        ];
        let g1 = build_task_groups(&running, &pins, None, true);
        assert_eq!(g1.len(), 2);
        assert_eq!(g1[0].key, "vivaldi.exe");
        assert!(g1[0].pinned);
        assert_eq!(g1[0].window_count, 1);

        // Closed: Vivaldi stays pinned with 0 windows
        let g2 = build_task_groups(&[], &pins, None, true);
        assert_eq!(g2.len(), 1);
        assert_eq!(g2[0].key, "vivaldi.exe");
        assert_eq!(g2[0].window_count, 0);
    }

    #[test]
    fn supports_ungrouped_mode_without_losing_windows() {
        let windows = vec![
            win(1, "vivaldi.exe", "A", "DISPLAY1", true),
            win(2, "vivaldi.exe", "B", "DISPLAY2", false),
        ];
        let groups = build_task_groups(&windows, &[], None, false);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].window_count, 1);
        assert_eq!(groups[1].window_count, 1);
    }
}
