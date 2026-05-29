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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum QualityGuardMode {
    Off,
    Warn,
    Skip,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SearchOrientationFilter {
    Any,
    Landscape,
    Portrait,
    Square,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SearchFilters {
    #[serde(default)]
    pub orientation: SearchOrientationFilter,
    #[serde(default)]
    pub min_width: u32,
    #[serde(default)]
    pub min_height: u32,
    #[serde(default)]
    pub color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HotkeySettings {
    #[serde(default = "default_next_wallpaper_hotkey")]
    pub next_wallpaper: String,
    #[serde(default = "default_pause_rotation_hotkey")]
    pub pause_rotation: String,
    #[serde(default = "default_favorite_current_hotkey")]
    pub favorite_current: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SupabaseSyncSettings {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub project_url: String,
    #[serde(default)]
    pub anon_key: String,
    #[serde(default)]
    pub use_clerk_auth: bool,
    #[serde(default = "default_supabase_sync_id")]
    pub sync_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ClerkAuthSettings {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub publishable_key: String,
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
    #[serde(default)]
    pub run_in_background: bool,
    #[serde(default)]
    pub launch_at_startup: bool,
    #[serde(default)]
    pub apply_to_lock_screen: bool,
    #[serde(default = "default_true")]
    pub global_hotkeys_enabled: bool,
    #[serde(default)]
    pub quality_guard_mode: QualityGuardMode,
    #[serde(default = "default_quality_min_width")]
    pub quality_min_width: u32,
    #[serde(default = "default_quality_min_height")]
    pub quality_min_height: u32,
    #[serde(default)]
    pub allow_portrait_wallpapers: bool,
    #[serde(default)]
    pub search_filters: SearchFilters,
    #[serde(default)]
    pub active_playlist_id: Option<String>,
    #[serde(default)]
    pub hotkeys: HotkeySettings,
    #[serde(default)]
    pub auto_clean_days: u64,
    #[serde(default = "default_true")]
    pub auto_clean_keep_favorites: bool,
    #[serde(default)]
    pub supabase_sync: SupabaseSyncSettings,
    #[serde(default)]
    pub clerk_auth: ClerkAuthSettings,
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
            run_in_background: false,
            launch_at_startup: false,
            apply_to_lock_screen: false,
            global_hotkeys_enabled: true,
            quality_guard_mode: QualityGuardMode::Warn,
            quality_min_width: default_quality_min_width(),
            quality_min_height: default_quality_min_height(),
            allow_portrait_wallpapers: false,
            search_filters: SearchFilters::default(),
            active_playlist_id: None,
            hotkeys: HotkeySettings::default(),
            auto_clean_days: 0,
            auto_clean_keep_favorites: true,
            supabase_sync: SupabaseSyncSettings::default(),
            clerk_auth: ClerkAuthSettings::default(),
        }
    }
}

impl Default for QualityGuardMode {
    fn default() -> Self {
        Self::Warn
    }
}

impl Default for SearchOrientationFilter {
    fn default() -> Self {
        Self::Any
    }
}

impl Default for SearchFilters {
    fn default() -> Self {
        Self {
            orientation: SearchOrientationFilter::Any,
            min_width: 0,
            min_height: 0,
            color: String::new(),
        }
    }
}

impl Default for HotkeySettings {
    fn default() -> Self {
        Self {
            next_wallpaper: default_next_wallpaper_hotkey(),
            pause_rotation: default_pause_rotation_hotkey(),
            favorite_current: default_favorite_current_hotkey(),
        }
    }
}

impl Default for SupabaseSyncSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            project_url: String::new(),
            anon_key: String::new(),
            use_clerk_auth: false,
            sync_id: default_supabase_sync_id(),
        }
    }
}

impl Default for ClerkAuthSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            publishable_key: String::new(),
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

impl ResolutionPreference {
    pub fn minimum_dimensions(self) -> (u32, u32) {
        match self {
            Self::Auto => (1280, 720),
            Self::FullHd => (1920, 1080),
            Self::FourK => (3840, 2160),
        }
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
        self.quality_min_width = self.quality_min_width.clamp(320, 15_360);
        self.quality_min_height = self.quality_min_height.clamp(240, 8_640);
        self.search_filters.min_width = self.search_filters.min_width.min(15_360);
        self.search_filters.min_height = self.search_filters.min_height.min(8_640);
        self.search_filters.color = self.search_filters.color.trim().to_string();
        self.hotkeys.next_wallpaper = normalize_hotkey(
            &self.hotkeys.next_wallpaper,
            &default_next_wallpaper_hotkey(),
        );
        self.hotkeys.pause_rotation = normalize_hotkey(
            &self.hotkeys.pause_rotation,
            &default_pause_rotation_hotkey(),
        );
        self.hotkeys.favorite_current = normalize_hotkey(
            &self.hotkeys.favorite_current,
            &default_favorite_current_hotkey(),
        );
        self.auto_clean_days = self.auto_clean_days.min(365);
        self.supabase_sync = self.supabase_sync.sanitized();
        self.clerk_auth = self.clerk_auth.sanitized();
        if self.auto_change_minutes > 0 {
            self.launch_at_startup = true;
        }
        if self.auto_change_minutes > 0 || self.launch_at_startup {
            self.run_in_background = true;
        }
        self
    }
}

impl SupabaseSyncSettings {
    pub fn sanitized(mut self) -> Self {
        self.project_url = normalize_supabase_project_url(&self.project_url);
        self.anon_key = self.anon_key.trim().to_string();
        self.sync_id = self.sync_id.trim().to_string();
        if self.use_clerk_auth {
            self.enabled = true;
        }
        if self.sync_id.is_empty() {
            self.sync_id = default_supabase_sync_id();
        }
        self
    }
}

impl ClerkAuthSettings {
    pub fn sanitized(mut self) -> Self {
        self.publishable_key = self.publishable_key.trim().to_string();
        if self.publishable_key.is_empty() {
            self.enabled = false;
        }
        self
    }
}

fn normalize_supabase_project_url(value: &str) -> String {
    let trimmed = value.trim().trim_end_matches('/');
    if let Some(project_ref) = supabase_ref_from_postgres_url(trimmed) {
        return format!("https://{project_ref}.supabase.co");
    }
    trimmed.to_string()
}

fn supabase_ref_from_postgres_url(value: &str) -> Option<String> {
    let rest = value
        .strip_prefix("postgresql://")
        .or_else(|| value.strip_prefix("postgres://"))?;
    let host = rest.split('@').nth(1)?.split([':', '/']).next()?;
    host.strip_prefix("db.")
        .and_then(|host| host.strip_suffix(".supabase.co"))
        .filter(|project_ref| {
            !project_ref.is_empty()
                && project_ref
                    .chars()
                    .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit())
        })
        .map(str::to_string)
}

fn default_quality_min_width() -> u32 {
    1920
}

fn default_quality_min_height() -> u32 {
    1080
}

fn default_true() -> bool {
    true
}

fn default_next_wallpaper_hotkey() -> String {
    "CommandOrControl+Alt+N".into()
}

fn default_pause_rotation_hotkey() -> String {
    "CommandOrControl+Alt+P".into()
}

fn default_favorite_current_hotkey() -> String {
    "CommandOrControl+Alt+F".into()
}

fn default_supabase_sync_id() -> String {
    "default".into()
}

fn normalize_hotkey(value: &str, fallback: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        fallback.to_string()
    } else {
        trimmed.to_string()
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
        assert!(!loaded.run_in_background);
        assert!(!loaded.launch_at_startup);
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
    fn saves_and_loads_background_startup_preferences() {
        let path = temp_settings_path("background-startup");
        let settings = AppSettings {
            run_in_background: true,
            launch_at_startup: true,
            ..AppSettings::default()
        };

        save_settings_to_path(&path, &settings).expect("settings should save");
        let loaded = load_settings_from_path(&path).expect("settings should load");

        assert!(loaded.run_in_background);
        assert!(loaded.launch_at_startup);

        let _ = fs::remove_file(path);
    }

    #[test]
    fn auto_change_interval_implies_background_runtime() {
        let settings = AppSettings {
            auto_change_minutes: 15,
            run_in_background: false,
            launch_at_startup: false,
            ..AppSettings::default()
        };

        let sanitized = settings.sanitized();

        assert!(sanitized.run_in_background);
        assert!(sanitized.launch_at_startup);
    }

    #[test]
    fn startup_launch_implies_background_runtime() {
        let settings = AppSettings {
            launch_at_startup: true,
            run_in_background: false,
            ..AppSettings::default()
        };

        assert!(settings.sanitized().run_in_background);
    }

    #[test]
    fn auto_change_minutes_are_clamped_to_one_day() {
        let settings = AppSettings {
            auto_change_minutes: 20_000,
            ..AppSettings::default()
        };

        assert_eq!(settings.sanitized().auto_change_minutes, 1_440);
    }

    #[test]
    fn resolution_preferences_map_to_minimum_dimensions() {
        assert_eq!(ResolutionPreference::Auto.minimum_dimensions(), (1280, 720));
        assert_eq!(
            ResolutionPreference::FullHd.minimum_dimensions(),
            (1920, 1080)
        );
        assert_eq!(
            ResolutionPreference::FourK.minimum_dimensions(),
            (3840, 2160)
        );
    }

    #[test]
    fn default_settings_enable_new_desktop_controls() {
        let settings = AppSettings::default();

        assert_eq!(settings.quality_guard_mode, QualityGuardMode::Warn);
        assert_eq!(settings.quality_min_width, 1920);
        assert_eq!(settings.quality_min_height, 1080);
        assert!(settings.global_hotkeys_enabled);
        assert!(!settings.apply_to_lock_screen);
        assert!(settings.auto_clean_keep_favorites);
        assert_eq!(settings.supabase_sync, SupabaseSyncSettings::default());
        assert_eq!(settings.clerk_auth, ClerkAuthSettings::default());
        assert_eq!(
            settings.search_filters.orientation,
            SearchOrientationFilter::Any
        );
        assert_eq!(settings.hotkeys.next_wallpaper, "CommandOrControl+Alt+N");
    }

    #[test]
    fn new_settings_fields_are_sanitized() {
        let settings = AppSettings {
            quality_min_width: 20,
            quality_min_height: 20_000,
            search_filters: SearchFilters {
                orientation: SearchOrientationFilter::Landscape,
                min_width: 90_000,
                min_height: 90_000,
                color: "  blue  ".into(),
            },
            hotkeys: HotkeySettings {
                next_wallpaper: " ".into(),
                pause_rotation: " Control+Shift+P ".into(),
                favorite_current: String::new(),
            },
            auto_clean_days: 800,
            supabase_sync: SupabaseSyncSettings {
                enabled: true,
                project_url: " https://example.supabase.co/ ".into(),
                anon_key: " anon-key ".into(),
                use_clerk_auth: true,
                sync_id: " ".into(),
            },
            clerk_auth: ClerkAuthSettings {
                enabled: true,
                publishable_key: " pk_test_example ".into(),
            },
            ..AppSettings::default()
        }
        .sanitized();

        assert_eq!(settings.quality_min_width, 320);
        assert_eq!(settings.quality_min_height, 8_640);
        assert_eq!(settings.search_filters.min_width, 15_360);
        assert_eq!(settings.search_filters.min_height, 8_640);
        assert_eq!(settings.search_filters.color, "blue");
        assert_eq!(settings.hotkeys.next_wallpaper, "CommandOrControl+Alt+N");
        assert_eq!(settings.hotkeys.pause_rotation, "Control+Shift+P");
        assert_eq!(settings.hotkeys.favorite_current, "CommandOrControl+Alt+F");
        assert_eq!(settings.auto_clean_days, 365);
        assert_eq!(
            settings.supabase_sync.project_url,
            "https://example.supabase.co"
        );
        assert_eq!(settings.supabase_sync.anon_key, "anon-key");
        assert!(settings.supabase_sync.enabled);
        assert!(settings.supabase_sync.use_clerk_auth);
        assert_eq!(settings.supabase_sync.sync_id, "default");
        assert_eq!(settings.clerk_auth.publishable_key, "pk_test_example");
    }

    #[test]
    fn supabase_sync_accepts_postgres_connection_string_as_project_url() {
        let settings = AppSettings {
            supabase_sync: SupabaseSyncSettings {
                enabled: true,
                project_url:
                    "postgresql://postgres:secret@db.fhqfqrdtdmoaxudbyyrb.supabase.co:5432/postgres"
                        .into(),
                anon_key: "anon-key".into(),
                use_clerk_auth: false,
                sync_id: "desktop".into(),
            },
            ..AppSettings::default()
        }
        .sanitized();

        assert_eq!(
            settings.supabase_sync.project_url,
            "https://fhqfqrdtdmoaxudbyyrb.supabase.co"
        );
        assert!(!settings.supabase_sync.project_url.contains("secret"));
    }

    #[test]
    fn older_settings_files_get_defaults_for_new_fields() {
        let path = temp_settings_path("legacy-fields");
        fs::write(
            &path,
            r#"{
              "apiKeys": {},
              "autoChangeMinutes": 0,
              "resolution": "auto",
              "cacheLimitMb": 1024
            }"#,
        )
        .expect("legacy settings should write");

        let loaded = load_settings_from_path(&path).expect("legacy settings should load");

        assert_eq!(loaded.quality_guard_mode, QualityGuardMode::Warn);
        assert!(loaded.global_hotkeys_enabled);
        assert_eq!(loaded.search_filters, SearchFilters::default());
        assert_eq!(loaded.hotkeys, HotkeySettings::default());
        assert_eq!(loaded.supabase_sync, SupabaseSyncSettings::default());

        let _ = fs::remove_file(path);
    }
}
