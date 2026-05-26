pub mod api;
pub mod cache;
pub mod models;
pub mod settings;
pub mod wallpaper;

use models::{ApiSource, CacheStats, Library, Wallpaper};
use reqwest::Client;
use settings::{
    load_settings_from_path, save_settings_to_path, settings_path, AppSettings,
    WallpaperLayoutPreference,
};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::async_runtime::{JoinHandle, Mutex};
use tauri::{Manager, State};

pub struct AppState {
    client: Client,
    settings_path: PathBuf,
    db_path: PathBuf,
    cache_dir: PathBuf,
    scheduler: Mutex<Option<JoinHandle<()>>>,
    wallpaper_lock: Arc<Mutex<Option<wallpaper::WallpaperLock>>>,
}

#[tauri::command]
fn get_settings(state: State<'_, AppState>) -> Result<AppSettings, String> {
    load_settings_from_path(&state.settings_path)
        .map_err(|error| format!("Could not load settings: {error}"))
}

#[tauri::command]
async fn save_settings(
    state: State<'_, AppState>,
    settings: AppSettings,
) -> Result<AppSettings, String> {
    let settings = settings.sanitized();
    save_settings_to_path(&state.settings_path, &settings)
        .map_err(|error| format!("Could not save settings: {error}"))?;
    restart_scheduler(&state, settings.auto_change_minutes).await?;
    Ok(settings)
}

#[tauri::command]
async fn search_wallpapers(
    state: State<'_, AppState>,
    query: String,
    page: u32,
    source: ApiSource,
) -> Result<Vec<Wallpaper>, String> {
    let settings = load_settings_from_path(&state.settings_path)
        .map_err(|error| format!("Could not load settings: {error}"))?;
    api::search_wallpapers(
        &state.client,
        &query,
        page,
        source,
        &settings.api_keys,
        settings.allow_nsfw_wallhaven,
    )
    .await
}

#[tauri::command]
async fn random_wallpapers(
    state: State<'_, AppState>,
    source: ApiSource,
) -> Result<Vec<Wallpaper>, String> {
    let settings = load_settings_from_path(&state.settings_path)
        .map_err(|error| format!("Could not load settings: {error}"))?;
    api::random_wallpapers(
        &state.client,
        source,
        &settings.api_keys,
        settings.allow_nsfw_wallhaven,
    )
    .await
}

#[tauri::command]
async fn set_wallpaper(
    state: State<'_, AppState>,
    wallpaper: Wallpaper,
) -> Result<Wallpaper, String> {
    let settings = load_settings_from_path(&state.settings_path)
        .map_err(|error| format!("Could not load settings: {error}"))?;
    let wallpaper = set_wallpaper_inner(
        &state.client,
        &state.cache_dir,
        &state.db_path,
        wallpaper,
        settings.wallpaper_layout,
    )
    .await?;
    remember_locked_wallpaper(&state.wallpaper_lock, &wallpaper, settings.wallpaper_layout).await;
    Ok(wallpaper)
}

#[tauri::command]
fn save_favorite(state: State<'_, AppState>, wallpaper: Wallpaper) -> Result<(), String> {
    cache::save_favorite(&state.db_path, &wallpaper)
}

#[tauri::command]
fn list_library(state: State<'_, AppState>) -> Result<Library, String> {
    cache::list_library(&state.db_path)
}

#[tauri::command]
fn cache_stats(state: State<'_, AppState>) -> Result<CacheStats, String> {
    cache::cache_stats(&state.cache_dir)
}

#[tauri::command]
fn clear_cache(state: State<'_, AppState>) -> Result<CacheStats, String> {
    cache::clear_cache(&state.cache_dir, &state.db_path)?;
    cache::cache_stats(&state.cache_dir)
}

#[tauri::command]
fn clear_library(state: State<'_, AppState>) -> Result<Library, String> {
    cache::clear_library(&state.db_path)?;
    cache::list_library(&state.db_path)
}

#[tauri::command]
async fn apply_random_wallpaper(state: State<'_, AppState>) -> Result<Wallpaper, String> {
    let wallpaper = apply_random_wallpaper_inner(
        state.client.clone(),
        state.settings_path.clone(),
        state.db_path.clone(),
        state.cache_dir.clone(),
    )
    .await?;
    let settings = load_settings_from_path(&state.settings_path)
        .map_err(|error| format!("Could not load settings: {error}"))?;
    remember_locked_wallpaper(&state.wallpaper_lock, &wallpaper, settings.wallpaper_layout).await;
    Ok(wallpaper)
}

async fn set_wallpaper_inner(
    client: &Client,
    cache_dir: &PathBuf,
    db_path: &PathBuf,
    mut wallpaper: Wallpaper,
    layout: WallpaperLayoutPreference,
) -> Result<Wallpaper, String> {
    let local_path = cache::download_wallpaper(client, cache_dir, &wallpaper).await?;
    let screen_path = wallpaper::prepare_wallpaper_for_screen(&local_path, cache_dir);
    wallpaper::set_desktop_wallpaper(&screen_path, layout)?;
    cache::record_wallpaper_used(db_path, &wallpaper, &screen_path)?;
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
    match api::random_wallpapers(
        &client,
        ApiSource::All,
        &settings.api_keys,
        settings.allow_nsfw_wallhaven,
    )
    .await
    {
        Ok(mut wallpapers) => {
            let wallpaper = wallpapers
                .drain(..)
                .next()
                .ok_or_else(|| "No random wallpapers were returned.".to_string())?;
            set_wallpaper_inner(
                &client,
                &cache_dir,
                &db_path,
                wallpaper,
                settings.wallpaper_layout,
            )
            .await
        }
        Err(error) => {
            if let Some(path) = cache::random_cached_wallpaper(&db_path)? {
                let screen_path = wallpaper::prepare_wallpaper_for_screen(&path, &cache_dir);
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
                    local_path: Some(screen_path.to_string_lossy().to_string()),
                    is_favorite: false,
                })
            } else {
                Err(error)
            }
        }
    }
}

async fn restart_scheduler(
    state: &State<'_, AppState>,
    interval_minutes: u64,
) -> Result<(), String> {
    let mut scheduler = state.scheduler.lock().await;
    if let Some(handle) = scheduler.take() {
        handle.abort();
    }

    if interval_minutes == 0 {
        return Ok(());
    }

    let client = state.client.clone();
    let settings_path = state.settings_path.clone();
    let db_path = state.db_path.clone();
    let cache_dir = state.cache_dir.clone();
    let wallpaper_lock = state.wallpaper_lock.clone();
    let interval = std::time::Duration::from_secs(interval_minutes * 60);

    *scheduler = Some(tauri::async_runtime::spawn(async move {
        let mut timer = tokio::time::interval(interval);
        loop {
            timer.tick().await;
            if let Ok(wallpaper) = apply_random_wallpaper_inner(
                client.clone(),
                settings_path.clone(),
                db_path.clone(),
                cache_dir.clone(),
            )
            .await
            {
                if let Ok(settings) = load_settings_from_path(&settings_path) {
                    remember_locked_wallpaper(
                        &wallpaper_lock,
                        &wallpaper,
                        settings.wallpaper_layout,
                    )
                    .await;
                }
            }
        }
    }));

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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let config_dir = app.path().app_config_dir()?;
            let data_dir = app.path().app_data_dir()?;
            let cache_dir = app.path().app_cache_dir()?.join("wallpapers");
            fs::create_dir_all(&config_dir)?;
            fs::create_dir_all(&data_dir)?;
            fs::create_dir_all(&cache_dir)?;

            let settings_path = settings_path(&config_dir);
            let settings = load_settings_from_path(&settings_path).unwrap_or_default();
            let db_path = data_dir.join("wallpapers.sqlite3");
            cache::init_database(&db_path)
                .map_err(|error| Box::<dyn std::error::Error>::from(error))?;
            let wallpaper_lock =
                Arc::new(Mutex::new(wallpaper::wallpaper_lock_from_current_desktop(
                    wallpaper::current_desktop_wallpaper().ok().flatten(),
                    settings.wallpaper_layout,
                )));
            start_wallpaper_guard(wallpaper_lock.clone());

            app.manage(AppState {
                client: Client::new(),
                settings_path,
                db_path,
                cache_dir,
                scheduler: Mutex::new(None),
                wallpaper_lock,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_settings,
            save_settings,
            search_wallpapers,
            random_wallpapers,
            set_wallpaper,
            save_favorite,
            list_library,
            cache_stats,
            clear_cache,
            clear_library,
            apply_random_wallpaper
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod config_tests {
    use serde_json::Value;

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
}
