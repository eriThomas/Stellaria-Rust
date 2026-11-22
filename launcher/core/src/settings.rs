use std::path::Path;

use serde::{Deserialize, Serialize};

pub const SCHEMA_VERSION: u32 = 1;
pub const MAX_BORDER_SIZE: u8 = 16;

pub const SUPPORTED_LANGUAGES: [&str; 13] = [
    "auto", "fr", "en", "es", "de", "pt-BR", "it", "pl", "ru", "tr", "zh-CN", "ja", "ko",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode {
    Dark,
    Light,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeVariant {
    TonalSpot,
    Vibrant,
    Expressive,
    Fidelity,
    Content,
    FruitSalad,
    Rainbow,
    Neutral,
    Monochrome,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardSettings {
    pub schema_version: u32,
    pub language: String,
    pub clock_seconds: bool,
    pub lyrics_enabled: bool,
    pub weather_enabled: bool,
    pub theme_mode: ThemeMode,
    pub theme_variant: ThemeVariant,
    pub source_color: String,
    pub reduced_motion: bool,
    pub game_mode: bool,
    pub border_size: u8,
}

impl Default for DashboardSettings {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            language: "auto".to_string(),
            clock_seconds: false,
            lyrics_enabled: true,
            weather_enabled: true,
            theme_mode: ThemeMode::Dark,
            theme_variant: ThemeVariant::TonalSpot,
            source_color: "#7e57c2".to_string(),
            reduced_motion: false,
            game_mode: false,
            border_size: 0,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardSettingsPatch {
    pub language: Option<String>,
    pub clock_seconds: Option<bool>,
    pub lyrics_enabled: Option<bool>,
    pub weather_enabled: Option<bool>,
    pub theme_mode: Option<ThemeMode>,
    pub theme_variant: Option<ThemeVariant>,
    pub source_color: Option<String>,
    pub reduced_motion: Option<bool>,
    pub game_mode: Option<bool>,
    pub border_size: Option<u8>,
}

fn is_valid_hex_color(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 7 && bytes[0] == b'#' && bytes[1..].iter().all(|b| b.is_ascii_hexdigit())
}

impl DashboardSettings {
    pub fn apply_patch(&mut self, patch: DashboardSettingsPatch) -> Result<(), String> {
        if let Some(language) = patch.language {
            if !SUPPORTED_LANGUAGES.contains(&language.as_str()) {
                return Err("unsupported-language".to_string());
            }
            self.language = language;
        }
        if let Some(color) = patch.source_color {
            if !is_valid_hex_color(&color) {
                return Err("invalid-color".to_string());
            }
            self.source_color = color.to_ascii_lowercase();
        }
        if let Some(v) = patch.clock_seconds {
            self.clock_seconds = v;
        }
        if let Some(v) = patch.lyrics_enabled {
            self.lyrics_enabled = v;
        }
        if let Some(v) = patch.weather_enabled {
            self.weather_enabled = v;
        }
        if let Some(v) = patch.theme_mode {
            self.theme_mode = v;
        }
        if let Some(v) = patch.theme_variant {
            self.theme_variant = v;
        }
        if let Some(v) = patch.reduced_motion {
            self.reduced_motion = v;
        }
        if let Some(v) = patch.game_mode {
            self.game_mode = v;
        }
        if let Some(v) = patch.border_size {
            self.border_size = v.min(MAX_BORDER_SIZE);
        }
        self.schema_version = SCHEMA_VERSION;
        Ok(())
    }

    pub fn load_or_default(path: &Path) -> Self {
        let Ok(raw) = std::fs::read_to_string(path) else {
            return Self::default();
        };
        let Ok(mut parsed) = serde_json::from_str::<Self>(&raw) else {
            return Self::default();
        };
        if !SUPPORTED_LANGUAGES.contains(&parsed.language.as_str()) {
            parsed.language = "auto".to_string();
        }
        if !is_valid_hex_color(&parsed.source_color) {
            parsed.source_color = "#7e57c2".to_string();
        }
        parsed.border_size = parsed.border_size.min(MAX_BORDER_SIZE);
        parsed.schema_version = SCHEMA_VERSION;
        parsed
    }

    pub fn save_atomic(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let payload = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        let temp = path.with_extension(format!("tmp.{}", std::process::id()));
        std::fs::write(&temp, payload).map_err(|e| e.to_string())?;
        std::fs::rename(&temp, path).map_err(|e| {
            let _ = std::fs::remove_file(&temp);
            e.to_string()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_invalid_language_and_color_without_mutating_state() {
        let mut settings = DashboardSettings::default();
        assert_eq!(
            settings.apply_patch(DashboardSettingsPatch {
                language: Some("xx-INVALID".into()),
                ..Default::default()
            }),
            Err("unsupported-language".into())
        );
        assert_eq!(settings.language, "auto");

        assert_eq!(
            settings.apply_patch(DashboardSettingsPatch {
                source_color: Some("javascript:alert(1)".into()),
                ..Default::default()
            }),
            Err("invalid-color".into())
        );
        assert_eq!(settings.source_color, "#7e57c2");
    }

    #[test]
    fn clamps_border_size_and_persists_atomically() {
        let dir = std::env::temp_dir().join(format!("stellaria-settings-test-{}", std::process::id()));
        let file = dir.join("settings.v1.json");
        let _ = std::fs::remove_file(&file);

        let mut settings = DashboardSettings::default();
        settings
            .apply_patch(DashboardSettingsPatch {
                language: Some("pt-BR".into()),
                clock_seconds: Some(true),
                border_size: Some(99),
                source_color: Some("#AABBCC".into()),
                ..Default::default()
            })
            .expect("valid patch");
        assert_eq!(settings.border_size, MAX_BORDER_SIZE);
        assert_eq!(settings.source_color, "#aabbcc");

        settings.save_atomic(&file).expect("save atomic");
        let loaded = DashboardSettings::load_or_default(&file);
        assert_eq!(loaded, settings);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
