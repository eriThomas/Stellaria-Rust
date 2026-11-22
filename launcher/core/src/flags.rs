use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AppIdentity {
    Production,
    Experimental,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeatureFlags {
    pub identity: AppIdentity,
    pub dashboard_pilot: bool,
    pub zebar_coexistence: bool,
}

impl FeatureFlags {
    pub fn from_env() -> Self {
        let experimental = std::env::var("STELLARIA_EXPERIMENTAL")
            .map(|v| matches!(v.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
            .unwrap_or(false)
            || cfg!(feature = "experimental");

        let dashboard_pilot = std::env::var("STELLARIA_DASHBOARD_PILOT")
            .map(|v| matches!(v.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
            .unwrap_or(experimental);

        Self {
            identity: if experimental {
                AppIdentity::Experimental
            } else {
                AppIdentity::Production
            },
            dashboard_pilot,
            zebar_coexistence: true,
        }
    }

    pub fn experimental() -> Self {
        Self {
            identity: AppIdentity::Experimental,
            dashboard_pilot: true,
            zebar_coexistence: true,
        }
    }

    pub fn production() -> Self {
        Self {
            identity: AppIdentity::Production,
            dashboard_pilot: false,
            zebar_coexistence: true,
        }
    }

    pub fn is_experimental(&self) -> bool {
        self.identity == AppIdentity::Experimental
    }
}

pub fn data_dir(local_app_data: &Path, identity: AppIdentity) -> PathBuf {
    match identity {
        AppIdentity::Production => local_app_data.join("Stellaria"),
        AppIdentity::Experimental => local_app_data.join("Stellaria-Experimental"),
    }
}

pub fn settings_path(local_app_data: &Path, identity: AppIdentity) -> PathBuf {
    data_dir(local_app_data, identity).join("settings.v1.json")
}

pub fn log_path(local_app_data: &Path, identity: AppIdentity) -> PathBuf {
    data_dir(local_app_data, identity).join("logs").join("shell.jsonl")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn experimental_data_dir_is_isolated_from_production() {
        let root = Path::new(r"C:\Users\tester\AppData\Local");
        let prod = data_dir(root, AppIdentity::Production);
        let exp = data_dir(root, AppIdentity::Experimental);
        assert_ne!(prod, exp);
        assert!(exp.ends_with("Stellaria-Experimental"));
        assert!(prod.ends_with("Stellaria"));
    }

    #[test]
    fn production_flags_disable_dashboard_pilot_by_default() {
        let flags = FeatureFlags::production();
        assert!(!flags.is_experimental());
        assert!(!flags.dashboard_pilot);
        assert!(flags.zebar_coexistence);
    }
}
