pub mod api;
pub mod app_log;
pub mod cache;
pub mod models;
pub mod quality;
pub mod settings;
pub mod sync;
pub mod wallpaper;

use models::{ApiSource, CacheStats, ImportResult, Library, Wallpaper, WallpaperQualityReport};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use settings::{
    load_settings_from_path, save_settings_to_path, settings_path, AppSettings, HotkeySettings,
    WallpaperLayoutPreference,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::async_runtime::{JoinHandle, Mutex};
use tauri::menu::MenuBuilder;
use tauri::tray::TrayIconBuilder;
use tauri::{Manager, State};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

const TRAY_OPEN_ID: &str = "open";
const TRAY_NEXT_ID: &str = "next-wallpaper";
const TRAY_PAUSE_ID: &str = "pause-rotation";
const TRAY_FAVORITE_ID: &str = "favorite-current";
const TRAY_QUIT_ID: &str = "quit";

const HOTKEY_NEXT_ID: &str = "next-wallpaper";
const HOTKEY_PAUSE_ID: &str = "pause-rotation";
const HOTKEY_FAVORITE_ID: &str = "favorite-current";

pub struct AppState {
    client: Client,
    settings_path: PathBuf,
    db_path: PathBuf,
    cache_dir: PathBuf,
    log_path: PathBuf,
    scheduler: Mutex<Option<JoinHandle<()>>>,
    scheduler_paused: Arc<AtomicBool>,
    wallpaper_lock: Arc<Mutex<Option<wallpaper::WallpaperLock>>>,
    startup_wallpaper: Arc<Mutex<Option<wallpaper::WallpaperLock>>>,
    explicit_exit_requested: AtomicBool,
}

impl AppState {
    fn log(&self, level: &str, target: &str, action: &str, message: &str, details: Value) {
        if let Err(error) =
            app_log::append_event(&self.log_path, level, target, action, message, details)
        {
            eprintln!("Could not write app log: {error}");
        }
    }

    fn log_backend(&self, action: &str, message: &str, details: Value) {
        self.log("info", "backend", action, message, details);
    }

    fn log_backend_warn(&self, action: &str, message: &str, details: Value) {
        self.log("warn", "backend", action, message, details);
    }

    fn log_backend_error(&self, action: &str, message: &str, details: Value) {
        self.log("error", "backend", action, message, details);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WindowCloseAction {
    HideToBackground,
    Exit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AppExitAction {
    KeepRunningInBackground,
    ExitProcess,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AppActivationMode {
    Foreground,
    Background,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BackupPayload {
    exported_at: String,
    settings: AppSettings,
    library: Library,
}

fn settings_log_details(settings: &AppSettings) -> Value {
    json!({
        "autoChangeMinutes": settings.auto_change_minutes,
        "resolution": format!("{:?}", settings.resolution),
        "cacheLimitMb": settings.cache_limit_mb,
        "theme": format!("{:?}", settings.theme),
        "wallpaperLayout": format!("{:?}", settings.wallpaper_layout),
        "runInBackground": settings.run_in_background,
        "launchAtStartup": settings.launch_at_startup,
        "applyToLockScreen": settings.apply_to_lock_screen,
        "globalHotkeysEnabled": settings.global_hotkeys_enabled,
        "qualityGuardMode": format!("{:?}", settings.quality_guard_mode),
        "qualityMinWidth": settings.quality_min_width,
        "qualityMinHeight": settings.quality_min_height,
        "allowPortraitWallpapers": settings.allow_portrait_wallpapers,
        "autoCleanDays": settings.auto_clean_days,
        "providersConfigured": {
            "pexels": !settings.api_keys.pexels.is_empty(),
            "unsplash": !settings.api_keys.unsplash.is_empty(),
            "pixabay": !settings.api_keys.pixabay.is_empty(),
            "wallhaven": !settings.api_keys.wallhaven.is_empty(),
            "deviantart": !settings.api_keys.deviantart.is_empty(),
        },
        "supabase": {
            "enabled": settings.supabase_sync.enabled,
            "projectConfigured": !settings.supabase_sync.project_url.is_empty(),
            "anonConfigured": !settings.supabase_sync.anon_key.is_empty(),
            "useClerkAuth": settings.supabase_sync.use_clerk_auth,
            "syncIdConfigured": !settings.supabase_sync.sync_id.is_empty(),
        },
        "clerk": {
            "enabled": settings.clerk_auth.enabled,
            "publicKeyConfigured": !settings.clerk_auth.publishable_key.is_empty(),
        }
    })
}

fn wallpaper_log_details(wallpaper: &Wallpaper) -> Value {
    json!({
        "id": &wallpaper.id,
        "source": &wallpaper.source,
        "width": wallpaper.width,
        "height": wallpaper.height,
        "queryUsed": &wallpaper.query_used,
        "mood": &wallpaper.mood,
        "hasLocalPath": wallpaper.local_path.is_some(),
        "isFavorite": wallpaper.is_favorite,
    })
}

fn library_log_details(library: &Library) -> Value {
    json!({
        "favorites": library.favorites.len(),
        "downloaded": library.downloaded.len(),
        "playlists": library.playlists.len(),
    })
}

fn cache_stats_log_details(stats: &CacheStats) -> Value {
    json!({
        "bytes": stats.bytes,
        "files": stats.files,
    })
}

fn log_command_error(state: &AppState, action: &str, error: String) -> String {
    state.log_backend_error(action, "Action failed.", json!({ "error": error }));
    error
}

fn log_app_event(app_handle: &tauri::AppHandle, action: &str, message: &str, details: Value) {
    if let Some(state) = app_handle.try_state::<AppState>() {
        state.log_backend(action, message, details);
    }
}

fn log_app_error(app_handle: &tauri::AppHandle, action: &str, message: &str, details: Value) {
    if let Some(state) = app_handle.try_state::<AppState>() {
        state.log_backend_error(action, message, details);
    }
}

#[tauri::command]
fn write_app_log(
    state: State<'_, AppState>,
    entry: app_log::FrontendLogEntry,
) -> Result<(), String> {
    app_log::append_frontend_entry(&state.log_path, &entry)
        .map_err(|error| format!("Could not write app log: {error}"))
}

#[tauri::command]
fn get_app_log_path(state: State<'_, AppState>) -> String {
    state.log_path.to_string_lossy().to_string()
}

#[tauri::command]
fn get_settings(state: State<'_, AppState>) -> Result<AppSettings, String> {
    match load_settings_from_path(&state.settings_path) {
        Ok(settings) => {
            state.log_backend(
                "settings.load",
                "Settings loaded.",
                settings_log_details(&settings),
            );
            Ok(settings)
        }
        Err(error) => Err(log_command_error(
            state.inner(),
            "settings.load",
            format!("Could not load settings: {error}"),
        )),
    }
}

#[tauri::command]
async fn save_settings(
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
    settings: AppSettings,
) -> Result<AppSettings, String> {
    let settings = settings.sanitized();
    state.log_backend(
        "settings.save.start",
        "Saving settings.",
        settings_log_details(&settings),
    );
    validate_hotkey_settings(&settings.hotkeys)?;
    sync_autostart(&app_handle, &settings)?;
    save_settings_to_path(&state.settings_path, &settings)
        .map_err(|error| format!("Could not save settings: {error}"))?;
    restart_scheduler(state.inner(), settings.auto_change_minutes).await?;
    if let Err(error) = configure_global_hotkeys(&app_handle, &settings) {
        eprintln!("Could not update global hotkeys: {error}");
        state.log_backend_warn(
            "hotkeys.configure",
            "Global hotkeys could not be updated after settings save.",
            json!({ "error": error.to_string() }),
        );
    }
    state.log_backend(
        "settings.save.success",
        "Settings saved.",
        settings_log_details(&settings),
    );
    Ok(settings)
}

#[tauri::command]
async fn search_wallpapers(
    state: State<'_, AppState>,
    query: String,
    page: u32,
    source: ApiSource,
) -> Result<Vec<Wallpaper>, String> {
    state.log_backend(
        "wallpaper.search.start",
        "Searching wallpapers.",
        json!({ "query": &query, "page": page, "source": format!("{:?}", source) }),
    );
    let settings = load_settings_from_path(&state.settings_path)
        .map_err(|error| format!("Could not load settings: {error}"))?;
    let result = api::search_wallpapers(
        &state.client,
        &query,
        page,
        source,
        &settings.api_keys,
        settings.allow_nsfw_wallhaven,
        settings.resolution,
    )
    .await;
    match &result {
        Ok(wallpapers) => state.log_backend(
            "wallpaper.search.success",
            "Wallpaper search completed.",
            json!({ "count": wallpapers.len(), "page": page, "source": format!("{:?}", source) }),
        ),
        Err(error) => state.log_backend_error(
            "wallpaper.search.error",
            "Wallpaper search failed.",
            json!({ "error": error }),
        ),
    }
    result
}

#[tauri::command]
async fn random_wallpapers(
    state: State<'_, AppState>,
    source: ApiSource,
) -> Result<Vec<Wallpaper>, String> {
    state.log_backend(
        "wallpaper.random.start",
        "Loading random wallpapers.",
        json!({ "source": format!("{:?}", source) }),
    );
    let settings = load_settings_from_path(&state.settings_path)
        .map_err(|error| format!("Could not load settings: {error}"))?;
    let result = api::random_wallpapers(
        &state.client,
        source,
        &settings.api_keys,
        settings.allow_nsfw_wallhaven,
        settings.resolution,
    )
    .await;
    match &result {
        Ok(wallpapers) => state.log_backend(
            "wallpaper.random.success",
            "Random wallpapers loaded.",
            json!({ "count": wallpapers.len(), "source": format!("{:?}", source) }),
        ),
        Err(error) => state.log_backend_error(
            "wallpaper.random.error",
            "Random wallpapers failed.",
            json!({ "error": error }),
        ),
    }
    result
}

#[tauri::command]
async fn set_wallpaper(
    state: State<'_, AppState>,
    wallpaper: Wallpaper,
) -> Result<Wallpaper, String> {
    state.log_backend(
        "wallpaper.apply.start",
        "Applying wallpaper.",
        wallpaper_log_details(&wallpaper),
    );
    let settings = load_settings_from_path(&state.settings_path)
        .map_err(|error| format!("Could not load settings: {error}"))?;
    let result = set_wallpaper_inner(
        &state.client,
        &state.cache_dir,
        &state.db_path,
        wallpaper,
        &settings,
        settings.wallpaper_layout,
    )
    .await;
    let wallpaper = match result {
        Ok(wallpaper) => wallpaper,
        Err(error) => {
            return Err(log_command_error(state.inner(), "wallpaper.apply", error));
        }
    };
    remember_locked_wallpaper(&state.wallpaper_lock, &wallpaper, settings.wallpaper_layout).await;
    state.log_backend(
        "wallpaper.apply.success",
        "Wallpaper applied.",
        wallpaper_log_details(&wallpaper),
    );
    Ok(wallpaper)
}

#[tauri::command]
async fn set_wallpaper_with_layout(
    state: State<'_, AppState>,
    wallpaper: Wallpaper,
    layout: WallpaperLayoutPreference,
) -> Result<Wallpaper, String> {
    state.log_backend(
        "wallpaper.apply.start",
        "Applying wallpaper with layout.",
        json!({ "wallpaper": wallpaper_log_details(&wallpaper), "layout": format!("{:?}", layout) }),
    );
    let settings = load_settings_from_path(&state.settings_path)
        .map_err(|error| format!("Could not load settings: {error}"))?;
    let result = set_wallpaper_inner(
        &state.client,
        &state.cache_dir,
        &state.db_path,
        wallpaper,
        &settings,
        layout,
    )
    .await;
    let wallpaper = match result {
        Ok(wallpaper) => wallpaper,
        Err(error) => {
            return Err(log_command_error(state.inner(), "wallpaper.apply", error));
        }
    };
    remember_locked_wallpaper(&state.wallpaper_lock, &wallpaper, layout).await;
    state.log_backend(
        "wallpaper.apply.success",
        "Wallpaper applied with layout.",
        json!({ "wallpaper": wallpaper_log_details(&wallpaper), "layout": format!("{:?}", layout) }),
    );
    Ok(wallpaper)
}

#[tauri::command]
fn assess_wallpaper_quality(
    state: State<'_, AppState>,
    wallpaper: Wallpaper,
) -> Result<WallpaperQualityReport, String> {
    let settings = load_settings_from_path(&state.settings_path)
        .map_err(|error| format!("Could not load settings: {error}"))?;
    let report = quality::assess_wallpaper_quality(&wallpaper, &settings);
    state.log_backend(
        "quality.assess",
        "Wallpaper quality assessed.",
        json!({
            "wallpaper": wallpaper_log_details(&wallpaper),
            "ok": report.ok,
            "warnings": &report.warnings,
        }),
    );
    Ok(report)
}

#[tauri::command]
fn set_lock_screen_wallpaper(
    state: State<'_, AppState>,
    wallpaper: Wallpaper,
) -> Result<(), String> {
    state.log_backend(
        "wallpaper.lock_screen.start",
        "Setting lock-screen wallpaper.",
        wallpaper_log_details(&wallpaper),
    );
    let settings = load_settings_from_path(&state.settings_path)
        .map_err(|error| format!("Could not load settings: {error}"))?;
    let path = wallpaper
        .local_path
        .as_deref()
        .or_else(|| {
            if Path::new(&wallpaper.full_url).exists() {
                Some(wallpaper.full_url.as_str())
            } else {
                None
            }
        })
        .ok_or_else(|| {
            "Download or apply this wallpaper before setting the lock screen.".to_string()
        })?;
    let result = wallpaper::set_lock_screen_wallpaper(Path::new(path), settings.wallpaper_layout);
    match &result {
        Ok(()) => state.log_backend(
            "wallpaper.lock_screen.success",
            "Lock-screen wallpaper updated.",
            wallpaper_log_details(&wallpaper),
        ),
        Err(error) => state.log_backend_error(
            "wallpaper.lock_screen.error",
            "Lock-screen wallpaper update failed.",
            json!({ "error": error }),
        ),
    }
    result
}

#[tauri::command]
fn save_favorite(state: State<'_, AppState>, wallpaper: Wallpaper) -> Result<(), String> {
    let result = cache::save_favorite(&state.db_path, &wallpaper);
    match &result {
        Ok(()) => state.log_backend(
            "library.favorite.save",
            "Wallpaper saved as favorite.",
            wallpaper_log_details(&wallpaper),
        ),
        Err(error) => state.log_backend_error(
            "library.favorite.save",
            "Saving favorite failed.",
            json!({ "error": error }),
        ),
    }
    result
}

#[tauri::command]
fn set_favorite(
    state: State<'_, AppState>,
    wallpaper: Wallpaper,
    favorite: bool,
) -> Result<Library, String> {
    state.log_backend(
        "library.favorite.toggle",
        "Favorite status changed.",
        json!({ "wallpaper": wallpaper_log_details(&wallpaper), "favorite": favorite }),
    );
    cache::set_favorite(&state.db_path, &wallpaper, favorite)?;
    let library = cache::list_library(&state.db_path)?;
    state.log_backend(
        "library.favorite.toggle.success",
        "Favorite status saved.",
        library_log_details(&library),
    );
    Ok(library)
}

#[tauri::command]
fn list_library(state: State<'_, AppState>) -> Result<Library, String> {
    let library = cache::list_library(&state.db_path)?;
    state.log_backend(
        "library.list",
        "Library loaded.",
        library_log_details(&library),
    );
    Ok(library)
}

#[tauri::command]
fn cache_stats(state: State<'_, AppState>) -> Result<CacheStats, String> {
    let stats = cache::cache_stats(&state.cache_dir)?;
    state.log_backend(
        "cache.stats",
        "Cache stats loaded.",
        cache_stats_log_details(&stats),
    );
    Ok(stats)
}

#[tauri::command]
fn clear_cache(state: State<'_, AppState>) -> Result<CacheStats, String> {
    state.log_backend("cache.clear.start", "Clearing wallpaper cache.", json!({}));
    cache::clear_cache(&state.cache_dir, &state.db_path)?;
    let stats = cache::cache_stats(&state.cache_dir)?;
    state.log_backend(
        "cache.clear.success",
        "Wallpaper cache cleared.",
        cache_stats_log_details(&stats),
    );
    Ok(stats)
}

#[tauri::command]
fn clear_library(state: State<'_, AppState>) -> Result<Library, String> {
    state.log_backend(
        "library.clear.start",
        "Clearing wallpaper library.",
        json!({}),
    );
    cache::clear_library(&state.db_path, &state.cache_dir)?;
    let library = cache::list_library(&state.db_path)?;
    state.log_backend(
        "library.clear.success",
        "Wallpaper library cleared.",
        library_log_details(&library),
    );
    Ok(library)
}

#[tauri::command]
fn delete_wallpaper(state: State<'_, AppState>, id: String) -> Result<Library, String> {
    state.log_backend(
        "library.wallpaper.delete.start",
        "Deleting wallpaper.",
        json!({ "id": &id }),
    );
    cache::delete_wallpaper(&state.db_path, &state.cache_dir, &id)?;
    let library = cache::list_library(&state.db_path)?;
    state.log_backend(
        "library.wallpaper.delete.success",
        "Wallpaper deleted.",
        library_log_details(&library),
    );
    Ok(library)
}

#[tauri::command]
fn create_playlist(state: State<'_, AppState>, name: String) -> Result<Library, String> {
    state.log_backend(
        "library.playlist.create.start",
        "Creating playlist.",
        json!({ "name": &name }),
    );
    let library = cache::create_playlist(&state.db_path, &name)?;
    state.log_backend(
        "library.playlist.create.success",
        "Playlist created.",
        library_log_details(&library),
    );
    Ok(library)
}

#[tauri::command]
fn delete_playlist(state: State<'_, AppState>, playlist_id: String) -> Result<Library, String> {
    state.log_backend(
        "library.playlist.delete.start",
        "Deleting playlist.",
        json!({ "playlistId": &playlist_id }),
    );
    let library = cache::delete_playlist(&state.db_path, &playlist_id)?;
    state.log_backend(
        "library.playlist.delete.success",
        "Playlist deleted.",
        library_log_details(&library),
    );
    Ok(library)
}

#[tauri::command]
fn add_wallpaper_to_playlist(
    state: State<'_, AppState>,
    playlist_id: String,
    wallpaper: Wallpaper,
) -> Result<Library, String> {
    state.log_backend(
        "library.playlist.add_wallpaper.start",
        "Adding wallpaper to playlist.",
        json!({ "playlistId": &playlist_id, "wallpaper": wallpaper_log_details(&wallpaper) }),
    );
    let library = cache::add_wallpaper_to_playlist(&state.db_path, &playlist_id, &wallpaper)?;
    state.log_backend(
        "library.playlist.add_wallpaper.success",
        "Wallpaper added to playlist.",
        library_log_details(&library),
    );
    Ok(library)
}

#[tauri::command]
fn remove_wallpaper_from_playlist(
    state: State<'_, AppState>,
    playlist_id: String,
    wallpaper_id: String,
) -> Result<Library, String> {
    state.log_backend(
        "library.playlist.remove_wallpaper.start",
        "Removing wallpaper from playlist.",
        json!({ "playlistId": &playlist_id, "wallpaperId": &wallpaper_id }),
    );
    let library =
        cache::remove_wallpaper_from_playlist(&state.db_path, &playlist_id, &wallpaper_id)?;
    state.log_backend(
        "library.playlist.remove_wallpaper.success",
        "Wallpaper removed from playlist.",
        library_log_details(&library),
    );
    Ok(library)
}

#[tauri::command]
fn import_local_folder(
    state: State<'_, AppState>,
    folder_path: String,
) -> Result<ImportResult, String> {
    state.log_backend(
        "library.import_folder.start",
        "Importing local wallpaper folder.",
        json!({ "folderPath": &folder_path }),
    );
    let result =
        cache::import_local_folder(&state.db_path, &state.cache_dir, Path::new(&folder_path));
    match &result {
        Ok(import) => state.log_backend(
            "library.import_folder.success",
            "Local wallpaper folder imported.",
            json!({ "imported": import.imported, "skipped": import.skipped }),
        ),
        Err(error) => state.log_backend_error(
            "library.import_folder.error",
            "Local wallpaper folder import failed.",
            json!({ "error": error }),
        ),
    }
    result
}

#[tauri::command]
fn run_auto_cleanup(state: State<'_, AppState>) -> Result<CacheStats, String> {
    state.log_backend(
        "cache.auto_cleanup.start",
        "Running auto-cleanup.",
        json!({}),
    );
    let settings = load_settings_from_path(&state.settings_path)
        .map_err(|error| format!("Could not load settings: {error}"))?;
    cache::cleanup_old_downloads(
        &state.db_path,
        &state.cache_dir,
        settings.auto_clean_days,
        settings.auto_clean_keep_favorites,
    )?;
    let stats = cache::cache_stats(&state.cache_dir)?;
    state.log_backend(
        "cache.auto_cleanup.success",
        "Auto-cleanup complete.",
        cache_stats_log_details(&stats),
    );
    Ok(stats)
}

#[tauri::command]
fn export_backup(state: State<'_, AppState>, target_path: String) -> Result<String, String> {
    state.log_backend(
        "backup.export.start",
        "Exporting backup.",
        json!({ "targetPath": &target_path }),
    );
    let settings = load_settings_from_path(&state.settings_path)
        .map_err(|error| format!("Could not load settings: {error}"))?;
    let payload = BackupPayload {
        exported_at: unix_timestamp_string(),
        settings,
        library: cache::list_library(&state.db_path)?,
    };
    let target = PathBuf::from(target_path);
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Could not create backup directory: {error}"))?;
    }
    fs::write(
        &target,
        serde_json::to_string_pretty(&payload)
            .map_err(|error| format!("Could not serialize backup: {error}"))?,
    )
    .map_err(|error| format!("Could not write backup: {error}"))?;
    let target = target.to_string_lossy().to_string();
    state.log_backend(
        "backup.export.success",
        "Backup exported.",
        json!({ "targetPath": &target }),
    );
    Ok(target)
}

#[tauri::command]
async fn import_backup(
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
    source_path: String,
) -> Result<Library, String> {
    state.log_backend(
        "backup.import.start",
        "Importing backup.",
        json!({ "sourcePath": &source_path }),
    );
    let raw = fs::read_to_string(&source_path)
        .map_err(|error| format!("Could not read backup: {error}"))?;
    let payload: BackupPayload =
        serde_json::from_str(&raw).map_err(|error| format!("Could not parse backup: {error}"))?;
    let settings = payload.settings.sanitized();
    sync_autostart(&app_handle, &settings)?;
    save_settings_to_path(&state.settings_path, &settings)
        .map_err(|error| format!("Could not restore settings: {error}"))?;
    restart_scheduler(state.inner(), settings.auto_change_minutes).await?;
    if let Err(error) = configure_global_hotkeys(&app_handle, &settings) {
        eprintln!("Could not update global hotkeys after backup import: {error}");
    }
    let mut wallpapers = payload.library.favorites.clone();
    wallpapers.extend(payload.library.downloaded.clone());
    for playlist in &payload.library.playlists {
        wallpapers.extend(playlist.wallpapers.clone());
    }
    cache::restore_wallpaper_metadata(&state.db_path, &wallpapers)?;
    cache::restore_playlists(&state.db_path, &payload.library.playlists)?;
    let library = cache::list_library(&state.db_path)?;
    state.log_backend(
        "backup.import.success",
        "Backup imported.",
        library_log_details(&library),
    );
    Ok(library)
}

#[tauri::command]
async fn test_supabase_sync(
    state: State<'_, AppState>,
    auth_context: Option<sync::SyncAuthContext>,
) -> Result<sync::SupabaseSyncStatus, String> {
    state.log_backend(
        "sync.test.start",
        "Testing Supabase sync connection.",
        json!({ "hasAuthContext": auth_context.is_some() }),
    );
    let settings = load_settings_from_path(&state.settings_path)
        .map_err(|error| format!("Could not load settings: {error}"))?;
    let result = sync::test_supabase_connection(
        &state.client,
        &settings.supabase_sync,
        auth_context.as_ref(),
    )
    .await;
    match &result {
        Ok(status) => state.log_backend(
            "sync.test.success",
            "Supabase sync test finished.",
            json!({ "connected": status.connected, "message": &status.message }),
        ),
        Err(error) => state.log_backend_error(
            "sync.test.error",
            "Supabase sync test failed.",
            json!({ "error": error }),
        ),
    }
    result
}

#[tauri::command]
async fn push_supabase_sync(
    state: State<'_, AppState>,
    auth_context: Option<sync::SyncAuthContext>,
) -> Result<sync::SupabaseSyncStatus, String> {
    state.log_backend(
        "sync.push.start",
        "Pushing sync snapshot.",
        json!({ "hasAuthContext": auth_context.is_some() }),
    );
    let settings = load_settings_from_path(&state.settings_path)
        .map_err(|error| format!("Could not load settings: {error}"))?;
    let library = cache::list_library(&state.db_path)?;
    let payload = sync::build_sync_payload(&settings, &library);
    let result = sync::push_supabase_sync(
        &state.client,
        &settings.supabase_sync,
        auth_context.as_ref(),
        &payload,
    )
    .await;
    match &result {
        Ok(status) => state.log_backend(
            "sync.push.success",
            "Sync snapshot pushed.",
            json!({
                "connected": status.connected,
                "message": &status.message,
                "library": library_log_details(&library),
            }),
        ),
        Err(error) => state.log_backend_error(
            "sync.push.error",
            "Sync snapshot push failed.",
            json!({ "error": error }),
        ),
    }
    result
}

#[tauri::command]
async fn pull_supabase_sync(
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
    auth_context: Option<sync::SyncAuthContext>,
) -> Result<sync::SupabaseSyncApplyResult, String> {
    state.log_backend(
        "sync.pull.start",
        "Pulling sync snapshot.",
        json!({ "hasAuthContext": auth_context.is_some() }),
    );
    let current_settings = load_settings_from_path(&state.settings_path)
        .map_err(|error| format!("Could not load settings: {error}"))?;
    let (payload, status) = sync::pull_supabase_sync(
        &state.client,
        &current_settings.supabase_sync,
        auth_context.as_ref(),
    )
    .await?;
    let mut settings = payload.settings.sanitized();
    settings.supabase_sync = current_settings.supabase_sync;
    sync_autostart(&app_handle, &settings)?;
    save_settings_to_path(&state.settings_path, &settings)
        .map_err(|error| format!("Could not save pulled settings: {error}"))?;
    restart_scheduler(state.inner(), settings.auto_change_minutes).await?;
    if let Err(error) = configure_global_hotkeys(&app_handle, &settings) {
        eprintln!("Could not update global hotkeys after Supabase pull: {error}");
    }

    let library = sync::portable_library_for_current_machine(payload.library);
    let wallpapers = sync::collect_library_wallpapers(&library);
    cache::restore_wallpaper_metadata(&state.db_path, &wallpapers)?;
    cache::restore_playlists(&state.db_path, &library.playlists)?;
    let library = cache::list_library(&state.db_path)?;

    state.log_backend(
        "sync.pull.success",
        "Sync snapshot pulled and applied.",
        json!({
            "connected": status.connected,
            "message": &status.message,
            "library": library_log_details(&library),
        }),
    );
    Ok(sync::SupabaseSyncApplyResult {
        status,
        settings,
        library,
    })
}

#[tauri::command]
async fn apply_random_wallpaper(state: State<'_, AppState>) -> Result<Wallpaper, String> {
    state.log_backend(
        "wallpaper.apply_random.start",
        "Applying random wallpaper.",
        json!({}),
    );
    let result = apply_random_wallpaper_inner(
        state.client.clone(),
        state.settings_path.clone(),
        state.db_path.clone(),
        state.cache_dir.clone(),
    )
    .await;
    let wallpaper = match result {
        Ok(wallpaper) => wallpaper,
        Err(error) => {
            return Err(log_command_error(
                state.inner(),
                "wallpaper.apply_random",
                error,
            ));
        }
    };
    let settings = load_settings_from_path(&state.settings_path)
        .map_err(|error| format!("Could not load settings: {error}"))?;
    remember_locked_wallpaper(&state.wallpaper_lock, &wallpaper, settings.wallpaper_layout).await;
    state.log_backend(
        "wallpaper.apply_random.success",
        "Random wallpaper applied.",
        wallpaper_log_details(&wallpaper),
    );
    Ok(wallpaper)
}

#[tauri::command]
async fn apply_next_wallpaper(state: State<'_, AppState>) -> Result<Wallpaper, String> {
    apply_random_wallpaper(state).await
}

#[tauri::command]
fn toggle_auto_change_pause(state: State<'_, AppState>) -> Result<bool, String> {
    let paused = !state.scheduler_paused.load(Ordering::SeqCst);
    state.scheduler_paused.store(paused, Ordering::SeqCst);
    state.log_backend(
        "scheduler.pause_toggle",
        "Auto-change pause toggled.",
        json!({ "paused": paused }),
    );
    Ok(paused)
}

#[tauri::command]
fn pause_global_hotkeys_for_capture(
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.log_backend(
        "hotkeys.capture.pause",
        "Pausing global hotkeys for capture.",
        json!({}),
    );
    let result = app_handle
        .global_shortcut()
        .unregister_all()
        .map_err(|error| format!("Could not pause global hotkeys: {error}"));
    if let Err(error) = &result {
        state.log_backend_error(
            "hotkeys.capture.pause",
            "Could not pause global hotkeys for capture.",
            json!({ "error": error }),
        );
    }
    result
}

#[tauri::command]
fn restore_global_hotkeys_after_capture(
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let settings = load_settings_from_path(&state.settings_path)
        .map_err(|error| format!("Could not load settings: {error}"))?;
    let result = configure_global_hotkeys(&app_handle, &settings);
    match &result {
        Ok(()) => state.log_backend(
            "hotkeys.capture.restore",
            "Global hotkeys restored after capture.",
            json!({}),
        ),
        Err(error) => state.log_backend_error(
            "hotkeys.capture.restore",
            "Could not restore global hotkeys after capture.",
            json!({ "error": error }),
        ),
    }
    result
}

async fn set_wallpaper_inner(
    client: &Client,
    cache_dir: &PathBuf,
    db_path: &PathBuf,
    mut wallpaper: Wallpaper,
    settings: &AppSettings,
    layout: WallpaperLayoutPreference,
) -> Result<Wallpaper, String> {
    let report = quality::assess_wallpaper_quality(&wallpaper, settings);
    if settings.quality_guard_mode == settings::QualityGuardMode::Skip && !report.ok {
        return Err(quality::quality_error_message(&report));
    }

    let local_path = wallpaper
        .local_path
        .as_deref()
        .map(PathBuf::from)
        .filter(|path| path.exists())
        .or_else(|| {
            let path = PathBuf::from(&wallpaper.full_url);
            path.exists().then_some(path)
        });
    let local_path = match local_path {
        Some(path) => path,
        None => cache::download_wallpaper(client, cache_dir, &wallpaper).await?,
    };
    let screen_path =
        wallpaper::prepare_wallpaper_for_screen(&local_path, cache_dir, settings.resolution);
    wallpaper::set_desktop_wallpaper(&screen_path, layout)?;
    if settings.apply_to_lock_screen {
        let _ = wallpaper::set_lock_screen_wallpaper(&screen_path, layout);
    }
    cache::record_wallpaper_used(db_path, &wallpaper, &screen_path)?;
    cache::enforce_cache_limit_mb(cache_dir, db_path, settings.cache_limit_mb)?;
    cache::cleanup_old_downloads(
        db_path,
        cache_dir,
        settings.auto_clean_days,
        settings.auto_clean_keep_favorites,
    )?;
    wallpaper.local_path = Some(screen_path.to_string_lossy().to_string());
    Ok(wallpaper)
}

async fn apply_random_wallpaper_inner(
    client: Client,
    settings_path: PathBuf,
    db_path: PathBuf,
    cache_dir: PathBuf,
) -> Result<Wallpaper, String> {
    let settings = load_settings_from_path(&settings_path)
        .map_err(|error| format!("Could not load settings: {error}"))?;
    if let Some(playlist_id) = settings.active_playlist_id.as_deref() {
        if let Some(wallpaper) = cache::random_playlist_wallpaper(&db_path, playlist_id)? {
            return set_wallpaper_inner(
                &client,
                &cache_dir,
                &db_path,
                wallpaper,
                &settings,
                settings.wallpaper_layout,
            )
            .await;
        }
    }

    match api::random_wallpapers(
        &client,
        ApiSource::All,
        &settings.api_keys,
        settings.allow_nsfw_wallhaven,
        settings.resolution,
    )
    .await
    {
        Ok(mut wallpapers) => {
            let wallpaper = wallpapers
                .drain(..)
                .find(|wallpaper| !quality::should_skip_wallpaper(wallpaper, &settings))
                .ok_or_else(|| "No random high-quality wallpapers were returned.".to_string())?;
            set_wallpaper_inner(
                &client,
                &cache_dir,
                &db_path,
                wallpaper,
                &settings,
                settings.wallpaper_layout,
            )
            .await
        }
        Err(error) => {
            if let Some(path) = cache::random_cached_wallpaper(&db_path)? {
                let screen_path =
                    wallpaper::prepare_wallpaper_for_screen(&path, &cache_dir, settings.resolution);
                wallpaper::set_desktop_wallpaper(&screen_path, settings.wallpaper_layout)?;
                Ok(Wallpaper {
                    id: screen_path
                        .file_stem()
                        .map(|stem| stem.to_string_lossy().to_string())
                        .unwrap_or_else(|| "cached-wallpaper".into()),
                    source: "cache".into(),
                    thumb_url: String::new(),
                    full_url: screen_path.to_string_lossy().to_string(),
                    photographer: "Local cache".into(),
                    width: 0,
                    height: 0,
                    query_used: Some("cache".into()),
                    mood: None,
                    local_path: Some(screen_path.to_string_lossy().to_string()),
                    is_favorite: false,
                })
            } else {
                Err(error)
            }
        }
    }
}

async fn restart_scheduler(state: &AppState, interval_minutes: u64) -> Result<(), String> {
    let mut scheduler = state.scheduler.lock().await;
    if let Some(handle) = scheduler.take() {
        handle.abort();
        state.log_backend(
            "scheduler.stop",
            "Existing auto-change scheduler stopped.",
            json!({}),
        );
    }

    let Some(interval) = scheduler_interval(interval_minutes) else {
        state.log_backend(
            "scheduler.disabled",
            "Auto-change scheduler disabled.",
            json!({ "intervalMinutes": interval_minutes }),
        );
        return Ok(());
    };

    let client = state.client.clone();
    let settings_path = state.settings_path.clone();
    let db_path = state.db_path.clone();
    let cache_dir = state.cache_dir.clone();
    let log_path = state.log_path.clone();
    let wallpaper_lock = state.wallpaper_lock.clone();
    let scheduler_paused = state.scheduler_paused.clone();
    state.log_backend(
        "scheduler.start",
        "Auto-change scheduler started.",
        json!({ "intervalMinutes": interval_minutes }),
    );

    *scheduler = Some(tauri::async_runtime::spawn(async move {
        let first_tick = tokio::time::Instant::from_std(scheduler_first_tick_at(
            std::time::Instant::now(),
            interval,
        ));
        let mut timer = tokio::time::interval_at(first_tick, interval);
        loop {
            timer.tick().await;
            if scheduler_paused.load(Ordering::SeqCst) {
                let _ = app_log::append_event(
                    &log_path,
                    "info",
                    "backend",
                    "scheduler.tick.skipped",
                    "Auto-change tick skipped while paused.",
                    json!({}),
                );
                continue;
            }
            let _ = app_log::append_event(
                &log_path,
                "info",
                "backend",
                "scheduler.tick",
                "Auto-change scheduler tick started.",
                json!({}),
            );
            match apply_random_wallpaper_inner(
                client.clone(),
                settings_path.clone(),
                db_path.clone(),
                cache_dir.clone(),
            )
            .await
            {
                Ok(wallpaper) => {
                    let _ = app_log::append_event(
                        &log_path,
                        "info",
                        "backend",
                        "scheduler.tick.success",
                        "Auto-change wallpaper applied.",
                        wallpaper_log_details(&wallpaper),
                    );
                    if let Ok(settings) = load_settings_from_path(&settings_path) {
                        remember_locked_wallpaper(
                            &wallpaper_lock,
                            &wallpaper,
                            settings.wallpaper_layout,
                        )
                        .await;
                    }
                }
                Err(error) => {
                    let _ = app_log::append_event(
                        &log_path,
                        "error",
                        "backend",
                        "scheduler.tick.error",
                        "Auto-change scheduler failed to apply wallpaper.",
                        json!({ "error": error }),
                    );
                }
            }
        }
    }));

    Ok(())
}

fn startup_scheduler_interval(settings: &AppSettings) -> Option<Duration> {
    scheduler_interval(settings.auto_change_minutes)
}

fn scheduler_interval(interval_minutes: u64) -> Option<Duration> {
    if interval_minutes == 0 {
        None
    } else {
        Some(Duration::from_secs(interval_minutes * 60))
    }
}

fn scheduler_first_tick_at(now: std::time::Instant, interval: Duration) -> std::time::Instant {
    now + interval
}

fn should_run_in_background(settings: &AppSettings) -> bool {
    settings.run_in_background || scheduler_interval(settings.auto_change_minutes).is_some()
}

fn desired_autostart_state(settings: &AppSettings) -> bool {
    settings.launch_at_startup && should_run_in_background(settings)
}

fn should_hide_minimized_window_to_tray(settings: &AppSettings) -> bool {
    should_run_in_background(settings)
}

fn window_close_action(settings: &AppSettings) -> WindowCloseAction {
    if should_hide_minimized_window_to_tray(settings) {
        WindowCloseAction::HideToBackground
    } else {
        WindowCloseAction::Exit
    }
}

fn app_exit_action(settings: &AppSettings, explicit_exit_requested: bool) -> AppExitAction {
    if explicit_exit_requested || !should_run_in_background(settings) {
        AppExitAction::ExitProcess
    } else {
        AppExitAction::KeepRunningInBackground
    }
}

fn activation_mode_for_visible_window(is_visible: bool) -> AppActivationMode {
    if is_visible {
        AppActivationMode::Foreground
    } else {
        AppActivationMode::Background
    }
}

fn launch_args_request_background<I, S>(args: I) -> bool
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    args.into_iter()
        .any(|arg| matches!(arg.as_ref(), "--background" | "--hidden" | "--tray"))
}

fn sync_autostart(app_handle: &tauri::AppHandle, settings: &AppSettings) -> Result<(), String> {
    let manager = app_handle.autolaunch();
    let desired = desired_autostart_state(settings);
    let enabled = manager
        .is_enabled()
        .map_err(|error| format!("Could not read startup app permission: {error}"))?;

    match (desired, enabled) {
        (true, false) => manager
            .enable()
            .map_err(|error| format!("Could not enable startup app permission: {error}"))?,
        (false, true) => manager
            .disable()
            .map_err(|error| format!("Could not disable startup app permission: {error}"))?,
        _ => {}
    }

    Ok(())
}

async fn remember_locked_wallpaper(
    wallpaper_lock: &Arc<Mutex<Option<wallpaper::WallpaperLock>>>,
    wallpaper: &Wallpaper,
    layout: WallpaperLayoutPreference,
) {
    if let Some(local_path) = wallpaper.local_path.as_deref() {
        let mut lock = wallpaper_lock.lock().await;
        *lock = Some(wallpaper::WallpaperLock {
            path: PathBuf::from(local_path),
            layout,
        });
    }
}

fn start_wallpaper_guard(wallpaper_lock: Arc<Mutex<Option<wallpaper::WallpaperLock>>>) {
    #[cfg(target_os = "windows")]
    {
        tauri::async_runtime::spawn(async move {
            let mut timer = tokio::time::interval(std::time::Duration::from_secs(5));
            loop {
                timer.tick().await;
                let lock = wallpaper_lock.lock().await.clone();
                if let Some(lock) = lock {
                    let current = wallpaper::current_desktop_wallpaper().ok().flatten();
                    let _ = wallpaper::restore_locked_wallpaper_if_needed(
                        &lock,
                        current,
                        wallpaper::set_desktop_wallpaper,
                    );
                }
            }
        });
    }

    #[cfg(not(target_os = "windows"))]
    let _ = wallpaper_lock;
}

fn restore_startup_wallpaper_before_exit(app_handle: &tauri::AppHandle) {
    let Some(state) = app_handle.try_state::<AppState>() else {
        return;
    };

    tauri::async_runtime::block_on(async {
        let startup_wallpaper = {
            let mut startup_wallpaper = state.startup_wallpaper.lock().await;
            startup_wallpaper.take()
        };
        let active_wallpaper_lock = state.wallpaper_lock.lock().await.clone();
        let current_wallpaper = wallpaper::current_desktop_wallpaper().ok().flatten();

        let _ = wallpaper::restore_startup_wallpaper_if_app_changed(
            startup_wallpaper.as_ref(),
            active_wallpaper_lock.as_ref(),
            current_wallpaper,
            wallpaper::set_desktop_wallpaper,
        );
    });
}

fn show_main_window(app_handle: &tauri::AppHandle) {
    log_app_event(app_handle, "window.show", "Showing main window.", json!({}));
    apply_activation_mode(app_handle, activation_mode_for_visible_window(true));
    if let Some(window) = app_handle.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn hide_main_window(app_handle: &tauri::AppHandle) {
    log_app_event(
        app_handle,
        "window.hide",
        "Hiding main window to background.",
        json!({}),
    );
    if let Some(window) = app_handle.get_webview_window("main") {
        let _ = window.hide();
    }
    apply_activation_mode(app_handle, activation_mode_for_visible_window(false));
}

fn apply_activation_mode(app_handle: &tauri::AppHandle, mode: AppActivationMode) {
    #[cfg(target_os = "macos")]
    {
        let policy = match mode {
            AppActivationMode::Foreground => tauri::ActivationPolicy::Regular,
            AppActivationMode::Background => tauri::ActivationPolicy::Accessory,
        };
        let _ = app_handle.set_activation_policy(policy);
    }

    #[cfg(not(target_os = "macos"))]
    let _ = (app_handle, mode);
}

#[cfg(target_os = "windows")]
fn hide_minimized_window_to_tray(app_handle: &tauri::AppHandle, label: &str) {
    let should_hide = app_handle
        .try_state::<AppState>()
        .and_then(|state| load_settings_from_path(&state.settings_path).ok())
        .map(|settings| should_hide_minimized_window_to_tray(&settings))
        .unwrap_or_default();

    if !should_hide {
        return;
    }

    if let Some(window) = app_handle.get_webview_window(label) {
        if window.is_minimized().unwrap_or(false) {
            log_app_event(
                app_handle,
                "window.minimize_to_tray",
                "Window minimized to tray.",
                json!({ "label": label }),
            );
            let _ = window.hide();
        }
    }
}

fn close_window_or_background_service(
    app_handle: &tauri::AppHandle,
    _label: &str,
    api: &tauri::CloseRequestApi,
) {
    let settings = app_handle
        .try_state::<AppState>()
        .and_then(|state| load_settings_from_path(&state.settings_path).ok())
        .unwrap_or_default();

    match window_close_action(&settings) {
        WindowCloseAction::HideToBackground => {
            log_app_event(
                app_handle,
                "window.close.background",
                "Close requested; app kept running in background.",
                json!({ "label": _label }),
            );
            api.prevent_close();
            hide_main_window(app_handle);
        }
        WindowCloseAction::Exit => {
            log_app_event(
                app_handle,
                "window.close.exit",
                "Close requested; app exiting.",
                json!({ "label": _label }),
            );
            app_handle.exit(0);
        }
    }
}

fn request_explicit_exit(app_handle: &tauri::AppHandle) {
    if let Some(state) = app_handle.try_state::<AppState>() {
        state.explicit_exit_requested.store(true, Ordering::SeqCst);
    }
    log_app_event(
        app_handle,
        "app.quit.request",
        "Explicit quit requested.",
        json!({}),
    );
    app_handle.exit(0);
}

fn handle_exit_requested(app_handle: &tauri::AppHandle, api: &tauri::ExitRequestApi) {
    let Some(state) = app_handle.try_state::<AppState>() else {
        return;
    };
    let settings = load_settings_from_path(&state.settings_path).unwrap_or_default();
    let explicit_exit = state.explicit_exit_requested.load(Ordering::SeqCst);

    match app_exit_action(&settings, explicit_exit) {
        AppExitAction::KeepRunningInBackground => {
            log_app_event(
                app_handle,
                "app.exit.background",
                "Exit requested; app kept running in background.",
                json!({ "explicitExit": explicit_exit }),
            );
            api.prevent_exit();
            hide_main_window(app_handle);
        }
        AppExitAction::ExitProcess => {
            log_app_event(
                app_handle,
                "app.exit.process",
                "App process exiting.",
                json!({ "explicitExit": explicit_exit }),
            );
            restore_startup_wallpaper_before_exit(app_handle);
        }
    }
}

fn apply_next_from_app_handle(app_handle: &tauri::AppHandle) {
    let Some(state) = app_handle.try_state::<AppState>() else {
        return;
    };
    state.log_backend(
        "quick_action.next.start",
        "Quick action requested next wallpaper.",
        json!({}),
    );
    let client = state.client.clone();
    let settings_path = state.settings_path.clone();
    let db_path = state.db_path.clone();
    let cache_dir = state.cache_dir.clone();
    let log_path = state.log_path.clone();
    let wallpaper_lock = state.wallpaper_lock.clone();

    tauri::async_runtime::spawn(async move {
        match apply_random_wallpaper_inner(client, settings_path.clone(), db_path, cache_dir).await
        {
            Ok(wallpaper) => {
                let _ = app_log::append_event(
                    &log_path,
                    "info",
                    "backend",
                    "quick_action.next.success",
                    "Quick action applied next wallpaper.",
                    wallpaper_log_details(&wallpaper),
                );
                if let Ok(settings) = load_settings_from_path(&settings_path) {
                    remember_locked_wallpaper(
                        &wallpaper_lock,
                        &wallpaper,
                        settings.wallpaper_layout,
                    )
                    .await;
                }
            }
            Err(error) => {
                let _ = app_log::append_event(
                    &log_path,
                    "error",
                    "backend",
                    "quick_action.next.error",
                    "Quick action failed to apply next wallpaper.",
                    json!({ "error": error }),
                );
            }
        }
    });
}

fn favorite_current_from_app_handle(app_handle: &tauri::AppHandle) {
    let Some(state) = app_handle.try_state::<AppState>() else {
        return;
    };
    if let Ok(Some(wallpaper)) = cache::last_used_wallpaper(&state.db_path) {
        let _ = cache::set_favorite(&state.db_path, &wallpaper, true);
        state.log_backend(
            "quick_action.favorite_current",
            "Quick action favorited current wallpaper.",
            wallpaper_log_details(&wallpaper),
        );
    } else {
        state.log_backend_warn(
            "quick_action.favorite_current",
            "Quick action could not find a current wallpaper to favorite.",
            json!({}),
        );
    }
}

fn toggle_scheduler_pause_from_app_handle(app_handle: &tauri::AppHandle) {
    let Some(state) = app_handle.try_state::<AppState>() else {
        return;
    };
    let paused = !state.scheduler_paused.load(Ordering::SeqCst);
    state.scheduler_paused.store(paused, Ordering::SeqCst);
    state.log_backend(
        "quick_action.pause_toggle",
        "Quick action toggled auto-change pause.",
        json!({ "paused": paused }),
    );
}

fn tray_menu_items() -> [(&'static str, &'static str); 5] {
    [
        (TRAY_OPEN_ID, "Open Wallpaper Engine"),
        (TRAY_NEXT_ID, "Next wallpaper"),
        (TRAY_PAUSE_ID, "Pause or resume auto-change"),
        (TRAY_FAVORITE_ID, "Favorite current wallpaper"),
        (TRAY_QUIT_ID, "Quit"),
    ]
}

fn parse_hotkey_setting(label: &str, value: &str) -> Result<Shortcut, String> {
    value
        .parse::<Shortcut>()
        .map_err(|error| format!("{label} hotkey is invalid: {error}"))
}

fn configured_hotkey_shortcuts(
    settings: &HotkeySettings,
) -> Result<Vec<(Shortcut, &'static str)>, String> {
    let shortcuts = vec![
        (
            parse_hotkey_setting("Next wallpaper", &settings.next_wallpaper)?,
            HOTKEY_NEXT_ID,
        ),
        (
            parse_hotkey_setting("Pause rotation", &settings.pause_rotation)?,
            HOTKEY_PAUSE_ID,
        ),
        (
            parse_hotkey_setting("Favorite current", &settings.favorite_current)?,
            HOTKEY_FAVORITE_ID,
        ),
    ];

    for (index, (shortcut, _)) in shortcuts.iter().enumerate() {
        if shortcuts
            .iter()
            .skip(index + 1)
            .any(|(candidate, _)| candidate == shortcut)
        {
            return Err("Hotkeys must be unique.".into());
        }
    }

    Ok(shortcuts)
}

fn validate_hotkey_settings(settings: &HotkeySettings) -> Result<(), String> {
    configured_hotkey_shortcuts(settings).map(|_| ())
}

fn configure_global_hotkeys(
    app_handle: &tauri::AppHandle,
    settings: &AppSettings,
) -> Result<(), String> {
    let manager = app_handle.global_shortcut();
    manager
        .unregister_all()
        .map_err(|error| format!("Could not reset global hotkeys: {error}"))?;

    if !settings.global_hotkeys_enabled {
        return Ok(());
    }

    let shortcuts = configured_hotkey_shortcuts(&settings.hotkeys)?;
    manager
        .register_multiple(shortcuts.into_iter().map(|(shortcut, _)| shortcut))
        .map_err(|error| format!("Could not register global hotkeys: {error}"))
}

fn global_hotkey_action(shortcut: &Shortcut, settings: &HotkeySettings) -> Option<&'static str> {
    configured_hotkey_shortcuts(settings)
        .ok()?
        .into_iter()
        .find_map(|(candidate, action)| (candidate == *shortcut).then_some(action))
}

fn handle_global_hotkey(app_handle: &tauri::AppHandle, shortcut: &Shortcut) {
    let Some(state) = app_handle.try_state::<AppState>() else {
        return;
    };
    let Ok(settings) = load_settings_from_path(&state.settings_path) else {
        return;
    };
    if !settings.global_hotkeys_enabled {
        return;
    }

    match global_hotkey_action(shortcut, &settings.hotkeys) {
        Some(HOTKEY_NEXT_ID) => {
            state.log_backend(
                "hotkey.next",
                "Global hotkey triggered next wallpaper.",
                json!({ "shortcut": format!("{:?}", shortcut) }),
            );
            apply_next_from_app_handle(app_handle);
        }
        Some(HOTKEY_PAUSE_ID) => {
            state.log_backend(
                "hotkey.pause",
                "Global hotkey toggled auto-change pause.",
                json!({ "shortcut": format!("{:?}", shortcut) }),
            );
            toggle_scheduler_pause_from_app_handle(app_handle);
        }
        Some(HOTKEY_FAVORITE_ID) => {
            state.log_backend(
                "hotkey.favorite_current",
                "Global hotkey favorited current wallpaper.",
                json!({ "shortcut": format!("{:?}", shortcut) }),
            );
            favorite_current_from_app_handle(app_handle);
        }
        _ => {}
    }
}

fn setup_global_hotkeys(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    app.handle().plugin(
        tauri_plugin_global_shortcut::Builder::new()
            .with_handler(|app_handle, shortcut, event| {
                if event.state == ShortcutState::Pressed {
                    handle_global_hotkey(app_handle, shortcut);
                }
            })
            .build(),
    )?;
    Ok(())
}

fn setup_tray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let mut menu = MenuBuilder::new(app);
    for (index, (id, label)) in tray_menu_items().into_iter().enumerate() {
        if index == 1 || index == 4 {
            menu = menu.separator();
        }
        menu = menu.text(id, label);
    }
    let menu = menu.build()?;
    let mut tray = TrayIconBuilder::with_id("main")
        .menu(&menu)
        .tooltip("Wallpaper Engine")
        .show_menu_on_left_click(true)
        .on_menu_event(|app_handle, event| match event.id().as_ref() {
            TRAY_OPEN_ID => {
                log_app_event(
                    app_handle,
                    "tray.open",
                    "Tray menu requested main window.",
                    json!({}),
                );
                show_main_window(app_handle);
            }
            TRAY_NEXT_ID => {
                log_app_event(
                    app_handle,
                    "tray.next",
                    "Tray menu requested next wallpaper.",
                    json!({}),
                );
                apply_next_from_app_handle(app_handle);
            }
            TRAY_PAUSE_ID => {
                log_app_event(
                    app_handle,
                    "tray.pause",
                    "Tray menu toggled auto-change pause.",
                    json!({}),
                );
                toggle_scheduler_pause_from_app_handle(app_handle);
            }
            TRAY_FAVORITE_ID => {
                log_app_event(
                    app_handle,
                    "tray.favorite_current",
                    "Tray menu favorited current wallpaper.",
                    json!({}),
                );
                favorite_current_from_app_handle(app_handle);
            }
            TRAY_QUIT_ID => {
                log_app_event(
                    app_handle,
                    "tray.quit",
                    "Tray menu requested quit.",
                    json!({}),
                );
                request_explicit_exit(app_handle);
            }
            _ => {}
        });

    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }

    tray.build(app)?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(target_os = "macos")]
    if std::env::current_exe()
        .ok()
        .as_deref()
        .is_some_and(is_macos_dmg_staging_executable)
    {
        return;
    }

    let mut builder = tauri::Builder::default();

    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(
            |app_handle, _argv, _cwd| {
                log_app_event(
                    app_handle,
                    "app.single_instance",
                    "Existing app instance received a second launch.",
                    json!({}),
                );
                show_main_window(app_handle);
            },
        ));
    }

    let app = builder
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--background".into()]),
        ))
        .setup(|app| {
            let config_dir = app.path().app_config_dir()?;
            let data_dir = app.path().app_data_dir()?;
            let cache_dir = app.path().app_cache_dir()?.join("wallpapers");
            let log_path = app_log::log_path(&data_dir);
            fs::create_dir_all(&config_dir)?;
            fs::create_dir_all(&data_dir)?;
            fs::create_dir_all(&cache_dir)?;
            let _ = app_log::append_event(
                &log_path,
                "info",
                "backend",
                "app.startup",
                "Wallpaper Engine startup began.",
                json!({ "logPath": log_path.to_string_lossy() }),
            );

            #[cfg(any(target_os = "linux", all(debug_assertions, windows)))]
            {
                use tauri_plugin_deep_link::DeepLinkExt;
                if let Err(error) = app.deep_link().register_all() {
                    eprintln!("Could not register deep links: {error}");
                    let _ = app_log::append_event(
                        &log_path,
                        "error",
                        "backend",
                        "deep_link.register",
                        "Could not register deep links.",
                        json!({ "error": error.to_string() }),
                    );
                }
            }

            let settings_path = settings_path(&config_dir);
            let settings = load_settings_from_path(&settings_path).unwrap_or_default();
            let db_path = data_dir.join("wallpapers.sqlite3");
            cache::init_database(&db_path)
                .map_err(|error| Box::<dyn std::error::Error>::from(error))?;
            let startup_interval = startup_scheduler_interval(&settings);
            let startup_wallpaper = wallpaper::wallpaper_lock_from_current_desktop(
                wallpaper::current_desktop_wallpaper().ok().flatten(),
                settings.wallpaper_layout,
            );
            let wallpaper_lock = Arc::new(Mutex::new(startup_wallpaper.clone()));
            let startup_wallpaper = Arc::new(Mutex::new(startup_wallpaper));
            start_wallpaper_guard(wallpaper_lock.clone());
            setup_tray(app)?;
            let start_hidden = launch_args_request_background(std::env::args())
                && should_run_in_background(&settings);

            app.manage(AppState {
                client: Client::new(),
                settings_path,
                db_path,
                cache_dir,
                log_path,
                scheduler: Mutex::new(None),
                scheduler_paused: Arc::new(AtomicBool::new(false)),
                wallpaper_lock,
                startup_wallpaper,
                explicit_exit_requested: AtomicBool::new(false),
            });
            if let Err(error) = setup_global_hotkeys(app) {
                eprintln!("Could not register global hotkeys: {error}");
                log_app_error(
                    app.handle(),
                    "hotkeys.plugin",
                    "Could not register global hotkey plugin.",
                    json!({ "error": error.to_string() }),
                );
            }
            if let Err(error) = configure_global_hotkeys(app.handle(), &settings) {
                eprintln!("Could not configure global hotkeys: {error}");
                log_app_error(
                    app.handle(),
                    "hotkeys.configure",
                    "Could not configure global hotkeys.",
                    json!({ "error": error.to_string() }),
                );
            }
            if let Some(interval) = startup_interval {
                let state = app.state::<AppState>();
                tauri::async_runtime::block_on(restart_scheduler(
                    state.inner(),
                    interval.as_secs() / 60,
                ))
                .map_err(|error| Box::<dyn std::error::Error>::from(error))?;
            }
            let _ = sync_autostart(app.handle(), &settings);
            log_app_event(
                app.handle(),
                "app.ready",
                "Wallpaper Engine startup completed.",
                settings_log_details(&settings),
            );
            if start_hidden {
                hide_main_window(app.handle());
            } else {
                show_main_window(app.handle());
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            write_app_log,
            get_app_log_path,
            get_settings,
            save_settings,
            search_wallpapers,
            random_wallpapers,
            set_wallpaper,
            set_wallpaper_with_layout,
            assess_wallpaper_quality,
            set_lock_screen_wallpaper,
            save_favorite,
            list_library,
            cache_stats,
            clear_cache,
            clear_library,
            delete_wallpaper,
            create_playlist,
            delete_playlist,
            add_wallpaper_to_playlist,
            remove_wallpaper_from_playlist,
            import_local_folder,
            run_auto_cleanup,
            export_backup,
            import_backup,
            test_supabase_sync,
            push_supabase_sync,
            pull_supabase_sync,
            set_favorite,
            apply_random_wallpaper,
            apply_next_wallpaper,
            toggle_auto_change_pause,
            pause_global_hotkeys_for_capture,
            restore_global_hotkeys_after_capture
        ])
        .build(tauri::generate_context!())
        .expect("error while running tauri application");

    app.run(|app_handle, event| match event {
        tauri::RunEvent::WindowEvent {
            label,
            event: tauri::WindowEvent::CloseRequested { api, .. },
            ..
        } => {
            close_window_or_background_service(app_handle, &label, &api);
        }
        #[cfg(target_os = "windows")]
        tauri::RunEvent::WindowEvent {
            label,
            event: tauri::WindowEvent::Resized(_),
            ..
        } => {
            hide_minimized_window_to_tray(app_handle, &label);
        }
        tauri::RunEvent::ExitRequested { api, .. } => {
            handle_exit_requested(app_handle, &api);
        }
        #[cfg(target_os = "macos")]
        tauri::RunEvent::Reopen { .. } => {
            show_main_window(app_handle);
        }
        _ => {}
    });
}

fn is_macos_dmg_staging_executable(path: &Path) -> bool {
    let parts = path
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>();

    parts
        .windows(2)
        .any(|window| window[0] == "Volumes" && window[1].starts_with("dmg."))
}

fn unix_timestamp_string() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(|_| "0".into())
}

#[cfg(test)]
mod config_tests {
    use super::*;
    use serde_json::Value;
    use std::time::{Duration, Instant};

    #[test]
    fn asset_protocol_allows_cached_wallpaper_previews() {
        let config: Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).expect("config is valid JSON");
        let asset_protocol = config
            .pointer("/app/security/assetProtocol")
            .expect("asset protocol is configured for local wallpaper previews");

        assert_eq!(
            asset_protocol.get("enable").and_then(Value::as_bool),
            Some(true)
        );
        let scope = asset_protocol
            .get("scope")
            .and_then(Value::as_array)
            .expect("asset protocol has a scope");

        assert!(
            scope
                .iter()
                .any(|entry| entry.as_str() == Some("$APPCACHE/wallpapers/**")),
            "asset protocol scope allows cached wallpapers"
        );
    }

    #[test]
    fn content_security_policy_is_restrictive_and_allows_wallpaper_images() {
        let config: Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).expect("config is valid JSON");
        let csp = config
            .pointer("/app/security/csp")
            .and_then(Value::as_str)
            .expect("content security policy is configured");

        assert!(csp.contains("default-src 'self'"));
        assert!(csp.contains("img-src 'self' asset: http://asset.localhost https: data:"));
        assert!(csp.contains("script-src 'self'"));
        assert!(csp.contains("https://*.clerk.accounts.dev"));
        assert!(csp.contains("frame-src https://*.clerk.accounts.dev https://*.clerk.com"));
    }

    #[test]
    fn desktop_deep_link_scheme_is_configured() {
        let config: Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).expect("config is valid JSON");
        let schemes = config
            .pointer("/plugins/deep-link/desktop/schemes")
            .and_then(Value::as_array)
            .expect("desktop deep-link schemes are configured");

        assert!(
            schemes
                .iter()
                .any(|entry| entry.as_str() == Some("wallpaper-engine")),
            "wallpaper-engine:// deep link scheme is registered"
        );
    }

    #[test]
    fn scheduler_first_tick_is_delayed_by_full_interval() {
        let now = Instant::now();
        let interval = Duration::from_secs(15 * 60);

        assert_eq!(scheduler_first_tick_at(now, interval), now + interval);
    }

    #[test]
    fn saved_auto_change_interval_starts_scheduler_on_launch() {
        let settings = AppSettings {
            auto_change_minutes: 15,
            ..AppSettings::default()
        };

        assert_eq!(
            startup_scheduler_interval(&settings),
            Some(Duration::from_secs(15 * 60))
        );
    }

    #[test]
    fn auto_change_settings_enable_startup_after_sanitization() {
        let settings = AppSettings {
            auto_change_minutes: 15,
            launch_at_startup: false,
            run_in_background: false,
            ..AppSettings::default()
        }
        .sanitized();

        assert!(desired_autostart_state(&settings));
    }

    #[test]
    fn zero_auto_change_interval_keeps_launch_scheduler_disabled() {
        let settings = AppSettings {
            auto_change_minutes: 0,
            ..AppSettings::default()
        };

        assert_eq!(startup_scheduler_interval(&settings), None);
    }

    #[test]
    fn closing_window_keeps_background_service_alive_when_auto_change_is_enabled() {
        let auto_change = AppSettings {
            auto_change_minutes: 15,
            ..AppSettings::default()
        };
        let background = AppSettings {
            run_in_background: true,
            ..AppSettings::default()
        };
        let disabled = AppSettings::default();

        assert_eq!(
            window_close_action(&auto_change),
            WindowCloseAction::HideToBackground
        );
        assert_eq!(
            window_close_action(&background),
            WindowCloseAction::HideToBackground
        );
        assert_eq!(window_close_action(&disabled), WindowCloseAction::Exit);
    }

    #[test]
    fn startup_autostart_is_enabled_only_after_permission() {
        let disabled = AppSettings::default();
        let background_without_startup = AppSettings {
            run_in_background: true,
            ..AppSettings::default()
        };
        let enabled = AppSettings {
            run_in_background: true,
            launch_at_startup: true,
            ..AppSettings::default()
        };

        assert_eq!(desired_autostart_state(&disabled), false);
        assert_eq!(desired_autostart_state(&background_without_startup), false);
        assert_eq!(desired_autostart_state(&enabled), true);
    }

    #[test]
    fn windows_minimize_to_tray_follows_background_permission() {
        let background = AppSettings {
            run_in_background: true,
            ..AppSettings::default()
        };
        let auto_change = AppSettings {
            auto_change_minutes: 10,
            ..AppSettings::default()
        };

        assert!(should_hide_minimized_window_to_tray(&background));
        assert!(should_hide_minimized_window_to_tray(&auto_change));
        assert!(!should_hide_minimized_window_to_tray(
            &AppSettings::default()
        ));
    }

    #[test]
    fn autostart_background_launch_uses_hidden_window_argument() {
        assert!(launch_args_request_background([
            "wallpaper-engine",
            "--background"
        ]));
        assert!(launch_args_request_background([
            "wallpaper-engine",
            "--hidden"
        ]));
        assert!(!launch_args_request_background(["wallpaper-engine"]));
    }

    #[test]
    fn os_level_quit_keeps_background_scheduler_alive() {
        let background = AppSettings {
            auto_change_minutes: 10,
            run_in_background: true,
            ..AppSettings::default()
        };

        assert_eq!(
            app_exit_action(&background, false),
            AppExitAction::KeepRunningInBackground
        );
    }

    #[test]
    fn explicit_tray_quit_is_the_real_exit_path() {
        let background = AppSettings {
            auto_change_minutes: 10,
            run_in_background: true,
            ..AppSettings::default()
        };

        assert_eq!(
            app_exit_action(&background, true),
            AppExitAction::ExitProcess
        );
    }

    #[test]
    fn tray_menu_exposes_background_quick_actions() {
        let items = tray_menu_items();
        let ids = items.map(|(id, _)| id);

        assert!(ids.contains(&TRAY_NEXT_ID));
        assert!(ids.contains(&TRAY_PAUSE_ID));
        assert!(ids.contains(&TRAY_FAVORITE_ID));
        assert!(ids.contains(&TRAY_QUIT_ID));
    }

    #[test]
    fn global_shortcuts_map_to_quick_actions() {
        let settings = HotkeySettings::default();

        for (shortcut, action) in
            configured_hotkey_shortcuts(&settings).expect("default hotkeys should parse")
        {
            assert_eq!(global_hotkey_action(&shortcut, &settings), Some(action));
        }
    }

    #[test]
    fn custom_global_shortcuts_map_to_quick_actions() {
        let settings = HotkeySettings {
            next_wallpaper: "Control+Shift+Right".into(),
            pause_rotation: "Control+Shift+Down".into(),
            favorite_current: "Control+Shift+F".into(),
        };
        let shortcuts =
            configured_hotkey_shortcuts(&settings).expect("custom hotkeys should parse");

        assert_eq!(
            global_hotkey_action(&shortcuts[0].0, &settings),
            Some(HOTKEY_NEXT_ID)
        );
        assert_eq!(
            global_hotkey_action(&shortcuts[1].0, &settings),
            Some(HOTKEY_PAUSE_ID)
        );
        assert_eq!(
            global_hotkey_action(&shortcuts[2].0, &settings),
            Some(HOTKEY_FAVORITE_ID)
        );
    }

    #[test]
    fn duplicate_global_shortcuts_are_rejected() {
        let settings = HotkeySettings {
            next_wallpaper: "Control+Shift+Right".into(),
            pause_rotation: "Control+Shift+Right".into(),
            favorite_current: "Control+Shift+F".into(),
        };

        assert!(configured_hotkey_shortcuts(&settings).is_err());
    }

    #[test]
    fn hidden_window_uses_background_activation_mode() {
        assert_eq!(
            activation_mode_for_visible_window(false),
            AppActivationMode::Background
        );
        assert_eq!(
            activation_mode_for_visible_window(true),
            AppActivationMode::Foreground
        );
    }

    #[test]
    fn bundled_window_starts_hidden_to_avoid_autostart_flash() {
        let config: Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).expect("config is valid JSON");
        let visible = config
            .pointer("/app/windows/0/visible")
            .and_then(Value::as_bool);

        assert_eq!(visible, Some(false));
    }

    #[test]
    fn bundled_window_allows_compact_resizing() {
        let config: Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).expect("config is valid JSON");
        let min_width = config
            .pointer("/app/windows/0/minWidth")
            .and_then(Value::as_u64);
        let min_height = config
            .pointer("/app/windows/0/minHeight")
            .and_then(Value::as_u64);

        assert!(min_width.is_some_and(|width| width <= 420));
        assert!(min_height.is_some_and(|height| height <= 560));
    }

    #[test]
    fn macos_dmg_packaging_staging_launch_exits_immediately() {
        assert!(is_macos_dmg_staging_executable(Path::new(
            "/Volumes/dmg.rclU6V/Wallpaper Engine.app/Contents/MacOS/wallpaper-engine"
        )));
        assert!(!is_macos_dmg_staging_executable(Path::new(
            "/Volumes/Wallpaper Engine/Wallpaper Engine.app/Contents/MacOS/wallpaper-engine"
        )));
        assert!(!is_macos_dmg_staging_executable(Path::new(
            "/Applications/Wallpaper Engine.app/Contents/MacOS/wallpaper-engine"
        )));
    }
}
