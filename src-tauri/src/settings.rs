use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ApiKeys {
    #[serde(default)]
    pub pexels: String,
    #[serde(default)]
    pub unsplash: String,
    #[serde(default)]
    pub pixabay: String,
    #[serde(default)]
    pub wallhaven: String,
    #[serde(default)]
    pub deviantart: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ResolutionPreference {
    Auto,
    FullHd,
    FourK,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ThemePreference {
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum WallpaperLayoutPreference {
    Fill,
    Fit,
    Stretch,
    Tile,
    Center,
    Span,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub api_keys: ApiKeys,
    pub auto_change_minutes: u64,
    pub resolution: ResolutionPreference,
    pub cache_limit_mb: u64,
    #[serde(default)]
    pub allow_nsfw_wallhaven: bool,
    #[serde(default)]
    pub theme: ThemePreference,
    #[serde(default)]
    pub wallpaper_layout: WallpaperLayoutPreference,
}

impl Default for ApiKeys {
    fn default() -> Self {
        Self {
            pexels: String::new(),
            unsplash: String::new(),
            pixabay: String::new(),
            wallhaven: String::new(),
            deviantart: String::new(),
        }
    }
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            api_keys: ApiKeys::default(),
            auto_change_minutes: 0,
            resolution: ResolutionPreference::Auto,
            cache_limit_mb: 1024,
            allow_nsfw_wallhaven: false,
            theme: ThemePreference::System,
            wallpaper_layout: WallpaperLayoutPreference::Fit,
        }
    }
}

impl Default for ThemePreference {
    fn default() -> Self {
        Self::System
    }
}

impl Default for WallpaperLayoutPreference {
    fn default() -> Self {
        Self::Fit
    }
}

impl AppSettings {
    pub fn sanitized(mut self) -> Self {
        self.api_keys.pexels = self.api_keys.pexels.trim().to_string();
        self.api_keys.unsplash = self.api_keys.unsplash.trim().to_string();
        self.api_keys.pixabay = self.api_keys.pixabay.trim().to_string();
        self.api_keys.wallhaven = self.api_keys.wallhaven.trim().to_string();
        self.api_keys.deviantart = self.api_keys.deviantart.trim().to_string();
        self.cache_limit_mb = self.cache_limit_mb.clamp(128, 10_240);
        self.auto_change_minutes = self.auto_change_minutes.min(1_440);
        self
    }
}

pub fn settings_path(config_dir: &Path) -> PathBuf {
    config_dir.join("settings.json")
}

pub fn load_settings_from_path(path: &Path) -> io::Result<AppSettings> {
    if !path.exists() {
        return Ok(AppSettings::default());
    }

    let raw = fs::read_to_string(path)?;
    if raw.trim().is_empty() {
        return Ok(AppSettings::default());
    }

    let settings = serde_json::from_str::<AppSettings>(&raw)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    Ok(settings.sanitized())
}

pub fn save_settings_to_path(path: &Path, settings: &AppSettings) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let settings = settings.clone().sanitized();
    let data = serde_json::to_string_pretty(&settings)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    fs::write(path, data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_settings_path(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("wallpaper-engine-{name}-{nanos}.json"))
    }

    #[test]
    fn saves_and_loads_api_keys() {
        let path = temp_settings_path("keys");
        let settings = AppSettings {
            api_keys: ApiKeys {
                pexels: "pexels-key".into(),
                unsplash: "unsplash-key".into(),
                pixabay: "pixabay-key".into(),
                wallhaven: "wallhaven-key".into(),
                deviantart: "deviantart-token".into(),
            },
            allow_nsfw_wallhaven: true,
            theme: ThemePreference::Dark,
            wallpaper_layout: WallpaperLayoutPreference::Span,
            ..AppSettings::default()
        };

        save_settings_to_path(&path, &settings).expect("settings should save");
        let loaded = load_settings_from_path(&path).expect("settings should load");

        assert_eq!(loaded.api_keys.pexels, "pexels-key");
        assert_eq!(loaded.api_keys.unsplash, "unsplash-key");
        assert_eq!(loaded.api_keys.pixabay, "pixabay-key");
        assert_eq!(loaded.api_keys.wallhaven, "wallhaven-key");
        assert_eq!(loaded.api_keys.deviantart, "deviantart-token");
        assert!(loaded.allow_nsfw_wallhaven);
        assert_eq!(loaded.theme, ThemePreference::Dark);
        assert_eq!(loaded.wallpaper_layout, WallpaperLayoutPreference::Span);

        let _ = fs::remove_file(path);
    }

    #[test]
    fn missing_settings_file_returns_defaults() {
        let path = temp_settings_path("missing");

        let loaded = load_settings_from_path(&path).expect("missing settings should load defaults");

        assert_eq!(loaded.api_keys.pexels, "");
        assert_eq!(loaded.api_keys.unsplash, "");
        assert_eq!(loaded.api_keys.pixabay, "");
        assert_eq!(loaded.api_keys.wallhaven, "");
        assert_eq!(loaded.api_keys.deviantart, "");
        assert_eq!(loaded.resolution, ResolutionPreference::Auto);
        assert_eq!(loaded.auto_change_minutes, 0);
        assert_eq!(loaded.cache_limit_mb, 1024);
        assert!(!loaded.allow_nsfw_wallhaven);
        assert_eq!(loaded.theme, ThemePreference::System);
        assert_eq!(loaded.wallpaper_layout, WallpaperLayoutPreference::Fit);
    }

    #[test]
    fn custom_auto_change_minutes_are_preserved_within_bounds() {
        let path = temp_settings_path("custom-auto-change");
        let settings = AppSettings {
            auto_change_minutes: 7,
            ..AppSettings::default()
        };

        save_settings_to_path(&path, &settings).expect("settings should save");
        let loaded = load_settings_from_path(&path).expect("settings should load");

        assert_eq!(loaded.auto_change_minutes, 7);

        let _ = fs::remove_file(path);
    }

    #[test]
    fn auto_change_minutes_are_clamped_to_one_day() {
        let settings = AppSettings {
            auto_change_minutes: 20_000,
            ..AppSettings::default()
        };

        assert_eq!(settings.sanitized().auto_change_minutes, 1_440);
    }
}
