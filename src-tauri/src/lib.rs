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
use std::time::Duration;
use tauri::async_runtime::{JoinHandle, Mutex};
use tauri::menu::MenuBuilder;
use tauri::tray::TrayIconBuilder;
use tauri::{Manager, State};
use tauri_plugin_autostart::ManagerExt;

const TRAY_OPEN_ID: &str = "open";
const TRAY_QUIT_ID: &str = "quit";

pub struct AppState {
    client: Client,
    settings_path: PathBuf,
    db_path: PathBuf,
    cache_dir: PathBuf,
    scheduler: Mutex<Option<JoinHandle<()>>>,
    wallpaper_lock: Arc<Mutex<Option<wallpaper::WallpaperLock>>>,
    startup_wallpaper: Arc<Mutex<Option<wallpaper::WallpaperLock>>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WindowCloseAction {
    HideToBackground,
    Exit,
}

#[tauri::command]
fn get_settings(state: State<'_, AppState>) -> Result<AppSettings, String> {
    load_settings_from_path(&state.settings_path)
        .map_err(|error| format!("Could not load settings: {error}"))
}

#[tauri::command]
async fn save_settings(
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
    settings: AppSettings,
) -> Result<AppSettings, String> {
    let settings = settings.sanitized();
    sync_autostart(&app_handle, &settings)?;
    save_settings_to_path(&state.settings_path, &settings)
        .map_err(|error| format!("Could not save settings: {error}"))?;
    restart_scheduler(state.inner(), settings.auto_change_minutes).await?;
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
        settings.resolution,
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
        settings.resolution,
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
        settings.resolution,
        settings.cache_limit_mb,
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
    resolution: settings::ResolutionPreference,
    cache_limit_mb: u64,
) -> Result<Wallpaper, String> {
    let local_path = cache::download_wallpaper(client, cache_dir, &wallpaper).await?;
    let screen_path = wallpaper::prepare_wallpaper_for_screen(&local_path, cache_dir, resolution);
    wallpaper::set_desktop_wallpaper(&screen_path, layout)?;
    cache::record_wallpaper_used(db_path, &wallpaper, &screen_path)?;
    cache::enforce_cache_limit_mb(cache_dir, db_path, cache_limit_mb)?;
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
        settings.resolution,
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
                settings.resolution,
                settings.cache_limit_mb,
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
    }

    let Some(interval) = scheduler_interval(interval_minutes) else {
        return Ok(());
    };

    let client = state.client.clone();
    let settings_path = state.settings_path.clone();
    let db_path = state.db_path.clone();
    let cache_dir = state.cache_dir.clone();
    let wallpaper_lock = state.wallpaper_lock.clone();

    *scheduler = Some(tauri::async_runtime::spawn(async move {
        let first_tick = tokio::time::Instant::from_std(scheduler_first_tick_at(
            std::time::Instant::now(),
            interval,
        ));
        let mut timer = tokio::time::interval_at(first_tick, interval);
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
    if let Some(window) = app_handle.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
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
            let _ = window.hide();
        }
    }
}

fn close_window_or_background_service(
    app_handle: &tauri::AppHandle,
    label: &str,
    api: &tauri::CloseRequestApi,
) {
    let settings = app_handle
        .try_state::<AppState>()
        .and_then(|state| load_settings_from_path(&state.settings_path).ok())
        .unwrap_or_default();

    match window_close_action(&settings) {
        WindowCloseAction::HideToBackground => {
            api.prevent_close();
            if let Some(window) = app_handle.get_webview_window(label) {
                let _ = window.hide();
            }
        }
        WindowCloseAction::Exit => {
            restore_startup_wallpaper_before_exit(app_handle);
            app_handle.exit(0);
        }
    }
}

fn setup_tray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let menu = MenuBuilder::new(app)
        .text(TRAY_OPEN_ID, "Open Wallpaper Engine")
        .separator()
        .text(TRAY_QUIT_ID, "Quit")
        .build()?;
    let mut tray = TrayIconBuilder::with_id("main")
        .menu(&menu)
        .tooltip("Wallpaper Engine")
        .show_menu_on_left_click(true)
        .on_menu_event(|app_handle, event| match event.id().as_ref() {
            TRAY_OPEN_ID => show_main_window(app_handle),
            TRAY_QUIT_ID => app_handle.exit(0),
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
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--background".into()]),
        ))
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
                scheduler: Mutex::new(None),
                wallpaper_lock,
                startup_wallpaper,
            });
            if let Some(interval) = startup_interval {
                let state = app.state::<AppState>();
                tauri::async_runtime::block_on(restart_scheduler(
                    state.inner(),
                    interval.as_secs() / 60,
                ))
                .map_err(|error| Box::<dyn std::error::Error>::from(error))?;
            }
            let _ = sync_autostart(app.handle(), &settings);
            if start_hidden {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                }
            }
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
        tauri::RunEvent::ExitRequested { .. } => {
            restore_startup_wallpaper_before_exit(app_handle);
        }
        #[cfg(target_os = "macos")]
        tauri::RunEvent::Reopen { .. } => {
            show_main_window(app_handle);
        }
        _ => {}
    });
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
}
