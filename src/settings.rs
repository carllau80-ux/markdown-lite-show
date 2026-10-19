use std::fs;
use std::path::PathBuf;

use eframe::egui::Color32;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    #[default]
    Zh,
    En,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemeMode {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccentColor {
    #[default]
    Teal,
    Ocean,
    Amber,
    Violet,
    Rose,
}

impl AccentColor {
    pub const ALL: [Self; 5] = [
        Self::Teal,
        Self::Ocean,
        Self::Amber,
        Self::Violet,
        Self::Rose,
    ];

    pub fn colors(self, dark: bool) -> AccentColors {
        use AccentColor::*;
        match (self, dark) {
            (Teal, false) => AccentColors::new(0x2f6f68, 0x2f6f68, 0xe3efec),
            (Teal, true) => AccentColors::new(0x8fc6bc, 0x35776d, 0x2a3b36),
            (Ocean, false) => AccentColors::new(0x2b6cb0, 0x2b6cb0, 0xe5effa),
            (Ocean, true) => AccentColors::new(0x8ab4f8, 0x2f5c9e, 0x22344a),
            (Amber, false) => AccentColors::new(0x946200, 0x946200, 0xfaf0d8),
            (Amber, true) => AccentColors::new(0xe7c66b, 0x806000, 0x3a3222),
            (Violet, false) => AccentColors::new(0x6b4eaa, 0x6b4eaa, 0xeee9f8),
            (Violet, true) => AccentColors::new(0xc4b5fd, 0x5b4699, 0x322b44),
            (Rose, false) => AccentColors::new(0xa23e5c, 0xa23e5c, 0xf8e7ed),
            (Rose, true) => AccentColors::new(0xf0a6bc, 0x8f3550, 0x3d2930),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct AccentColors {
    pub accent: Color32,
    pub button: Color32,
    pub soft: Color32,
}

impl AccentColors {
    const fn new(accent: u32, button: u32, soft: u32) -> Self {
        Self {
            accent: rgb(accent),
            button: rgb(button),
            soft: rgb(soft),
        }
    }
}

const fn rgb(hex: u32) -> Color32 {
    Color32::from_rgb(
        ((hex >> 16) & 0xff) as u8,
        ((hex >> 8) & 0xff) as u8,
        (hex & 0xff) as u8,
    )
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub language: Language,
    pub theme: ThemeMode,
    pub accent: AccentColor,
}

impl Settings {
    pub fn load() -> Self {
        config_path()
            .and_then(|path| fs::read_to_string(path).ok())
            .and_then(|contents| serde_json::from_str(&contents).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) -> Result<(), String> {
        let path = config_path().ok_or_else(|| "user config directory not found".to_owned())?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        let contents = serde_json::to_string_pretty(self).map_err(|error| error.to_string())?;
        fs::write(path, contents).map_err(|error| error.to_string())
    }
}

pub fn config_path() -> Option<PathBuf> {
    let base = directories::BaseDirs::new()?;
    Some(base.config_dir().join("MarkLite").join("settings.json"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_follow_system_theme() {
        let settings = Settings::default();
        assert_eq!(settings.language, Language::Zh);
        assert_eq!(settings.theme, ThemeMode::System);
        assert_eq!(settings.accent, AccentColor::Teal);
    }

    #[test]
    fn settings_round_trip() {
        let settings = Settings {
            language: Language::En,
            theme: ThemeMode::Dark,
            accent: AccentColor::Ocean,
        };
        let json = serde_json::to_string(&settings).unwrap();
        let decoded: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, settings);
    }
}
