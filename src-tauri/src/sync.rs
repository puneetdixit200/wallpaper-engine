use crate::models::{Library, Wallpaper, WallpaperPlaylist};
use crate::settings::{AppSettings, SupabaseSyncSettings};
use reqwest::{Client, RequestBuilder};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

const SYNC_TABLE: &str = "wallpaper_engine_sync";
const SYNC_PAYLOAD_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SupabaseSyncStatus {
    pub connected: bool,
    pub message: String,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SyncAuthContext {
    pub access_token: String,
    pub user_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedSyncAuth {
    pub config: SupabaseSyncSettings,
    pub row_id: String,
    pub bearer_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SupabaseSyncApplyResult {
    pub status: SupabaseSyncStatus,
    pub settings: AppSettings,
    pub library: Library,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SyncPayload {
    pub version: u32,
    pub synced_at: String,
    pub settings: AppSettings,
    pub library: Library,
}

#[derive(Debug, Serialize)]
struct SyncUpsertRow<'a> {
    id: &'a str,
    updated_at: &'a str,
    payload: &'a SyncPayload,
}

#[derive(Debug, Deserialize)]
struct SyncStatusRow {
    updated_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SyncPayloadRow {
    updated_at: Option<String>,
    payload: SyncPayload,
}

pub fn build_sync_payload(settings: &AppSettings, library: &Library) -> SyncPayload {
    SyncPayload {
        version: SYNC_PAYLOAD_VERSION,
        synced_at: unix_timestamp_string(),
        settings: settings_without_supabase_credentials(settings),
        library: library.clone(),
    }
}

pub fn settings_without_supabase_credentials(settings: &AppSettings) -> AppSettings {
    let mut settings = settings.clone().sanitized();
    settings.supabase_sync = SupabaseSyncSettings::default();
    settings
}

pub fn portable_library_for_current_machine(library: Library) -> Library {
    Library {
        favorites: strip_missing_local_paths(library.favorites),
        downloaded: strip_missing_local_paths(library.downloaded),
        playlists: library
            .playlists
            .into_iter()
            .map(|playlist| WallpaperPlaylist {
                id: playlist.id,
                name: playlist.name,
                wallpapers: strip_missing_local_paths(playlist.wallpapers),
            })
            .collect(),
    }
}

pub fn collect_library_wallpapers(library: &Library) -> Vec<Wallpaper> {
    let mut wallpapers = library.favorites.clone();
    wallpapers.extend(library.downloaded.clone());
    for playlist in &library.playlists {
        wallpapers.extend(playlist.wallpapers.clone());
    }
    wallpapers
}

pub fn validate_supabase_credentials(
    config: &SupabaseSyncSettings,
) -> Result<SupabaseSyncSettings, String> {
    let config = config.clone().sanitized();
    if config.project_url.is_empty() {
        return Err("Supabase project URL is required.".into());
    }
    if !is_supported_project_url(&config.project_url) {
        return Err("Supabase project URL must use HTTPS, localhost, or 127.0.0.1.".into());
    }
    if config.anon_key.is_empty() {
        return Err("Supabase anon key is required.".into());
    }
    if !config.use_clerk_auth && (config.sync_id.len() > 128 || !is_valid_sync_id(&config.sync_id))
    {
        return Err("Sync ID can only use letters, numbers, dots, dashes, and underscores.".into());
    }
    Ok(config)
}

pub fn require_enabled_config(
    config: &SupabaseSyncSettings,
) -> Result<SupabaseSyncSettings, String> {
    let config = validate_supabase_credentials(config)?;
    if !config.enabled {
        return Err("Enable Supabase sync before pushing or pulling.".into());
    }
    Ok(config)
}

pub fn resolve_sync_auth(
    config: &SupabaseSyncSettings,
    auth_context: Option<&SyncAuthContext>,
    require_enabled: bool,
) -> Result<ResolvedSyncAuth, String> {
    let config = if require_enabled {
        require_enabled_config(config)?
    } else {
        validate_supabase_credentials(config)?
    };

    if config.use_clerk_auth {
        let auth_context = auth_context
            .ok_or_else(|| "Sign in with Clerk before using Supabase sync.".to_string())?;
        let user_id = auth_context.user_id.trim();
        if user_id.len() > 128 || !is_valid_sync_id(user_id) {
            return Err("Clerk user ID from the current session is invalid.".into());
        }
        let access_token = auth_context.access_token.trim();
        if access_token.is_empty() {
            return Err("Clerk session token is missing. Sign in again and retry.".into());
        }
        return Ok(ResolvedSyncAuth {
            config,
            row_id: user_id.to_string(),
            bearer_token: access_token.to_string(),
        });
    }

    Ok(ResolvedSyncAuth {
        row_id: config.sync_id.clone(),
        bearer_token: config.anon_key.clone(),
        config,
    })
}

pub async fn test_supabase_connection(
    client: &Client,
    config: &SupabaseSyncSettings,
    auth_context: Option<&SyncAuthContext>,
) -> Result<SupabaseSyncStatus, String> {
    let auth = resolve_sync_auth(config, auth_context, false)?;
    let rows: Vec<SyncStatusRow> = send_supabase_json(
        supabase_headers(
            client.get(row_query_url(&auth.config, &auth.row_id, "updated_at")),
            &auth,
        ),
        "Supabase sync test failed",
    )
    .await?;
    Ok(status_from_rows(
        rows,
        "Supabase connection works.",
        "Supabase connection works. No cloud snapshot yet.",
    ))
}

pub async fn push_supabase_sync(
    client: &Client,
    config: &SupabaseSyncSettings,
    auth_context: Option<&SyncAuthContext>,
    payload: &SyncPayload,
) -> Result<SupabaseSyncStatus, String> {
    let auth = resolve_sync_auth(config, auth_context, true)?;
    let rows = vec![SyncUpsertRow {
        id: &auth.row_id,
        updated_at: &payload.synced_at,
        payload,
    }];
    let saved_rows: Vec<SyncStatusRow> = send_supabase_json(
        supabase_headers(client.post(table_url(&auth.config)), &auth)
            .header(
                "Prefer",
                "resolution=merge-duplicates,return=representation",
            )
            .json(&rows),
        "Supabase sync push failed",
    )
    .await?;
    Ok(status_from_rows(
        saved_rows,
        "Cloud snapshot saved.",
        "Cloud snapshot saved.",
    ))
}

pub async fn pull_supabase_sync(
    client: &Client,
    config: &SupabaseSyncSettings,
    auth_context: Option<&SyncAuthContext>,
) -> Result<(SyncPayload, SupabaseSyncStatus), String> {
    let auth = resolve_sync_auth(config, auth_context, true)?;
    let rows: Vec<SyncPayloadRow> = send_supabase_json(
        supabase_headers(
            client.get(row_query_url(
                &auth.config,
                &auth.row_id,
                "updated_at,payload",
            )),
            &auth,
        ),
        "Supabase sync pull failed",
    )
    .await?;
    let row = rows
        .into_iter()
        .next()
        .ok_or_else(|| "No cloud snapshot was found for this account.".to_string())?;
    let status = SupabaseSyncStatus {
        connected: true,
        message: "Cloud snapshot pulled.".into(),
        updated_at: row.updated_at.clone(),
    };
    Ok((row.payload, status))
}

fn status_from_rows(
    rows: Vec<SyncStatusRow>,
    found_message: &str,
    empty_message: &str,
) -> SupabaseSyncStatus {
    if let Some(row) = rows.into_iter().next() {
        SupabaseSyncStatus {
            connected: true,
            message: found_message.into(),
            updated_at: row.updated_at,
        }
    } else {
        SupabaseSyncStatus {
            connected: true,
            message: empty_message.into(),
            updated_at: None,
        }
    }
}

async fn send_supabase_json<T>(request: RequestBuilder, context: &str) -> Result<T, String>
where
    T: for<'de> Deserialize<'de>,
{
    let response = request
        .send()
        .await
        .map_err(|error| format!("{context}: {error}"))?;
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|error| format!("{context}: could not read response body: {error}"))?;
    if !status.is_success() {
        return Err(format!("{context} ({status}): {body}"));
    }
    serde_json::from_str(&body).map_err(|error| format!("{context}: invalid response: {error}"))
}

fn supabase_headers(request: RequestBuilder, auth: &ResolvedSyncAuth) -> RequestBuilder {
    request
        .header("apikey", &auth.config.anon_key)
        .bearer_auth(&auth.bearer_token)
        .header("Content-Type", "application/json")
}

pub fn table_url(config: &SupabaseSyncSettings) -> String {
    format!("{}/rest/v1/{}", config.project_url, SYNC_TABLE)
}

pub fn row_query_url(config: &SupabaseSyncSettings, row_id: &str, select: &str) -> String {
    format!("{}?id=eq.{}&select={}", table_url(config), row_id, select)
}

fn strip_missing_local_paths(wallpapers: Vec<Wallpaper>) -> Vec<Wallpaper> {
    wallpapers
        .into_iter()
        .map(|mut wallpaper| {
            if wallpaper
                .local_path
                .as_deref()
                .is_some_and(|path| !Path::new(path).exists())
            {
                wallpaper.local_path = None;
            }
            wallpaper
        })
        .collect()
}

fn is_supported_project_url(value: &str) -> bool {
    value.starts_with("https://")
        || value.starts_with("http://localhost")
        || value.starts_with("http://127.0.0.1")
}

fn is_valid_sync_id(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_'))
}

fn unix_timestamp_string() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(|_| "0".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_wallpaper(local_path: Option<String>) -> Wallpaper {
        Wallpaper {
            id: "pexels-42".into(),
            source: "pexels".into(),
            thumb_url: "https://images.pexels.com/thumb.jpg".into(),
            full_url: "https://images.pexels.com/full.jpg".into(),
            photographer: "Example".into(),
            width: 3840,
            height: 2160,
            query_used: Some("forest".into()),
            mood: Some("nature".into()),
            local_path,
            is_favorite: true,
        }
    }

    fn sync_config() -> SupabaseSyncSettings {
        SupabaseSyncSettings {
            enabled: true,
            project_url: "https://project.supabase.co".into(),
            anon_key: "anon".into(),
            use_clerk_auth: false,
            sync_id: "desktop-main".into(),
        }
    }

    #[test]
    fn validates_supabase_sync_config() {
        let config = sync_config();

        assert!(validate_supabase_credentials(&config).is_ok());
        assert!(validate_supabase_credentials(&SupabaseSyncSettings {
            sync_id: "bad id".into(),
            ..config.clone()
        })
        .is_err());
        assert!(validate_supabase_credentials(&SupabaseSyncSettings {
            project_url: "ftp://project.supabase.co".into(),
            ..config
        })
        .is_err());
    }

    #[test]
    fn builds_supabase_rest_urls() {
        let config = sync_config();

        assert_eq!(
            table_url(&config),
            "https://project.supabase.co/rest/v1/wallpaper_engine_sync"
        );
        assert_eq!(
            row_query_url(&config, &config.sync_id, "updated_at,payload"),
            "https://project.supabase.co/rest/v1/wallpaper_engine_sync?id=eq.desktop-main&select=updated_at,payload"
        );
    }

    #[test]
    fn resolves_manual_sync_auth() {
        let auth = resolve_sync_auth(&sync_config(), None, true).expect("auth should resolve");

        assert_eq!(auth.row_id, "desktop-main");
        assert_eq!(auth.bearer_token, "anon");
    }

    #[test]
    fn resolves_clerk_sync_auth() {
        let config = SupabaseSyncSettings {
            use_clerk_auth: true,
            sync_id: String::new(),
            ..sync_config()
        };
        let auth = resolve_sync_auth(
            &config,
            Some(&SyncAuthContext {
                access_token: "clerk-token".into(),
                user_id: "user_123".into(),
            }),
            true,
        )
        .expect("auth should resolve");

        assert_eq!(auth.row_id, "user_123");
        assert_eq!(auth.bearer_token, "clerk-token");
    }

    #[test]
    fn requires_clerk_session_in_clerk_mode() {
        let config = SupabaseSyncSettings {
            use_clerk_auth: true,
            ..sync_config()
        };

        assert!(resolve_sync_auth(&config, None, true).is_err());
    }

    #[test]
    fn scrubs_supabase_credentials_from_sync_payload() {
        let settings = AppSettings {
            supabase_sync: sync_config(),
            ..AppSettings::default()
        };
        let payload = build_sync_payload(
            &settings,
            &Library {
                favorites: Vec::new(),
                downloaded: Vec::new(),
                playlists: Vec::new(),
            },
        );

        assert_eq!(
            payload.settings.supabase_sync,
            SupabaseSyncSettings::default()
        );
    }

    #[test]
    fn removes_missing_local_paths_from_pulled_library() {
        let library = Library {
            favorites: vec![sample_wallpaper(Some("/missing/wallpaper.jpg".into()))],
            downloaded: Vec::new(),
            playlists: vec![WallpaperPlaylist {
                id: "playlist".into(),
                name: "Playlist".into(),
                wallpapers: vec![sample_wallpaper(Some("/missing/playlist.jpg".into()))],
            }],
        };

        let portable = portable_library_for_current_machine(library);

        assert_eq!(portable.favorites[0].local_path, None);
        assert_eq!(portable.playlists[0].wallpapers[0].local_path, None);
    }
}
