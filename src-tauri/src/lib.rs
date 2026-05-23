pub mod api;
pub mod cache;
pub mod models;
pub mod settings;
pub mod wallpaper;

use models::{ApiSource, CacheStats, Library, Wallpaper};
use reqwest::Client;
use settings::{load_settings_from_path, save_settings_to_path, settings_path, AppSettings};
use std::fs;
use std::path::PathBuf;
use tauri::async_runtime::{JoinHandle, Mutex};
use tauri::{Manager, State};

pub struct AppState {
    client: Client,
    settings_path: PathBuf,
    db_path: PathBuf,
    cache_dir: PathBuf,
    scheduler: Mutex<Option<JoinHandle<()>>>,
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
    api::search_wallpapers(&state.client, &query, page, source, &settings.api_keys).await
}

#[tauri::command]
async fn random_wallpapers(
    state: State<'_, AppState>,
    source: ApiSource,
) -> Result<Vec<Wallpaper>, String> {
    let settings = load_settings_from_path(&state.settings_path)
        .map_err(|error| format!("Could not load settings: {error}"))?;
    api::random_wallpapers(&state.client, source, &settings.api_keys).await
}

#[tauri::command]
async fn set_wallpaper(
    state: State<'_, AppState>,
    wallpaper: Wallpaper,
) -> Result<Wallpaper, String> {
    set_wallpaper_inner(
        &state.client,
        &state.cache_dir,
        &state.db_path,
        wallpaper,
    )
    .await
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
async fn apply_random_wallpaper(state: State<'_, AppState>) -> Result<Wallpaper, String> {
    apply_random_wallpaper_inner(
        state.client.clone(),
        state.settings_path.clone(),
        state.db_path.clone(),
        state.cache_dir.clone(),
    )
    .await
}

async fn set_wallpaper_inner(
    client: &Client,
    cache_dir: &PathBuf,
    db_path: &PathBuf,
    mut wallpaper: Wallpaper,
) -> Result<Wallpaper, String> {
    let local_path = cache::download_wallpaper(client, cache_dir, &wallpaper).await?;
    wallpaper::set_desktop_wallpaper(&local_path)?;
    cache::record_wallpaper_used(db_path, &wallpaper, &local_path)?;
    wallpaper.local_path = Some(local_path.to_string_lossy().to_string());
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
    match api::random_wallpapers(&client, ApiSource::Both, &settings.api_keys).await {
        Ok(mut wallpapers) => {
            let wallpaper = wallpapers
                .drain(..)
                .next()
                .ok_or_else(|| "No random wallpapers were returned.".to_string())?;
            set_wallpaper_inner(&client, &cache_dir, &db_path, wallpaper).await
        }
        Err(error) => {
            if let Some(path) = cache::random_cached_wallpaper(&db_path)? {
                wallpaper::set_desktop_wallpaper(&path)?;
                Ok(Wallpaper {
                    id: path
                        .file_stem()
                        .map(|stem| stem.to_string_lossy().to_string())
                        .unwrap_or_else(|| "cached-wallpaper".into()),
                    source: "cache".into(),
                    thumb_url: String::new(),
                    full_url: path.to_string_lossy().to_string(),
                    photographer: "Local cache".into(),
                    width: 0,
                    height: 0,
                    query_used: Some("cache".into()),
                    local_path: Some(path.to_string_lossy().to_string()),
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
    let interval = std::time::Duration::from_secs(interval_minutes * 60);

    *scheduler = Some(tauri::async_runtime::spawn(async move {
        let mut timer = tokio::time::interval(interval);
        loop {
            timer.tick().await;
            let _ = apply_random_wallpaper_inner(
                client.clone(),
                settings_path.clone(),
                db_path.clone(),
                cache_dir.clone(),
            )
            .await;
        }
    }));

    Ok(())
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

            let db_path = data_dir.join("wallpapers.sqlite3");
            cache::init_database(&db_path)
                .map_err(|error| Box::<dyn std::error::Error>::from(error))?;

            app.manage(AppState {
                client: Client::new(),
                settings_path: settings_path(&config_dir),
                db_path,
                cache_dir,
                scheduler: Mutex::new(None),
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
            apply_random_wallpaper
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
