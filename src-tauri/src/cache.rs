use crate::models::{CacheStats, Library, Wallpaper};
use reqwest::Client;
use rusqlite::{params, Connection};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn init_database(db_path: &Path) -> Result<(), String> {
    if let Some(parent) = db_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Could not create database directory: {error}"))?;
    }

    let connection = Connection::open(db_path)
        .map_err(|error| format!("Could not open wallpaper database: {error}"))?;
    connection
        .execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS wallpapers (
                id          TEXT PRIMARY KEY,
                source      TEXT NOT NULL,
                url_thumb   TEXT NOT NULL,
                url_full    TEXT NOT NULL,
                photographer TEXT NOT NULL DEFAULT '',
                width       INTEGER NOT NULL DEFAULT 0,
                height      INTEGER NOT NULL DEFAULT 0,
                local_path  TEXT,
                query_used  TEXT,
                mood        TEXT,
                is_favorite INTEGER NOT NULL DEFAULT 0,
                used_count  INTEGER NOT NULL DEFAULT 0,
                last_used   TEXT,
                created_at  TEXT NOT NULL
            );
            "#,
        )
        .map_err(|error| format!("Could not initialize wallpaper database: {error}"))?;
    Ok(())
}

pub fn save_favorite(db_path: &Path, wallpaper: &Wallpaper) -> Result<(), String> {
    upsert_wallpaper(db_path, wallpaper, None, true, false)
}

pub fn upsert_downloaded_wallpaper(
    db_path: &Path,
    wallpaper: &Wallpaper,
    local_path: &Path,
) -> Result<(), String> {
    upsert_wallpaper(db_path, wallpaper, Some(local_path), false, false)
}

pub fn record_wallpaper_used(
    db_path: &Path,
    wallpaper: &Wallpaper,
    local_path: &Path,
) -> Result<(), String> {
    upsert_wallpaper(db_path, wallpaper, Some(local_path), false, true)
}

fn upsert_wallpaper(
    db_path: &Path,
    wallpaper: &Wallpaper,
    local_path: Option<&Path>,
    favorite: bool,
    used: bool,
) -> Result<(), String> {
    init_database(db_path)?;
    let connection = Connection::open(db_path)
        .map_err(|error| format!("Could not open wallpaper database: {error}"))?;
    let now = unix_timestamp_string();
    let local_path = local_path.map(|path| path.to_string_lossy().to_string());
    let used_count_delta = if used { 1_i64 } else { 0_i64 };
    let last_used = if used { Some(now.clone()) } else { None };

    connection
        .execute(
            r#"
            INSERT INTO wallpapers (
                id, source, url_thumb, url_full, photographer, width, height,
                local_path, query_used, is_favorite, used_count, last_used, created_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
            ON CONFLICT(id) DO UPDATE SET
                source = excluded.source,
                url_thumb = excluded.url_thumb,
                url_full = excluded.url_full,
                photographer = excluded.photographer,
                width = excluded.width,
                height = excluded.height,
                local_path = COALESCE(excluded.local_path, wallpapers.local_path),
                query_used = COALESCE(excluded.query_used, wallpapers.query_used),
                is_favorite = CASE
                    WHEN wallpapers.is_favorite = 1 OR excluded.is_favorite = 1 THEN 1
                    ELSE 0
                END,
                used_count = wallpapers.used_count + excluded.used_count,
                last_used = COALESCE(excluded.last_used, wallpapers.last_used)
            "#,
            params![
                wallpaper.id,
                wallpaper.source,
                wallpaper.thumb_url,
                wallpaper.full_url,
                wallpaper.photographer,
                wallpaper.width,
                wallpaper.height,
                local_path,
                wallpaper.query_used,
                if favorite { 1_i64 } else { 0_i64 },
                used_count_delta,
                last_used,
                now,
            ],
        )
        .map_err(|error| format!("Could not save wallpaper metadata: {error}"))?;

    Ok(())
}

pub fn list_library(db_path: &Path) -> Result<Library, String> {
    init_database(db_path)?;
    Ok(Library {
        favorites: query_wallpapers(db_path, "WHERE is_favorite = 1 ORDER BY created_at DESC")?,
        downloaded: query_wallpapers(
            db_path,
            "WHERE local_path IS NOT NULL ORDER BY last_used DESC, created_at DESC",
        )?,
    })
}

fn query_wallpapers(db_path: &Path, clause: &str) -> Result<Vec<Wallpaper>, String> {
    let connection = Connection::open(db_path)
        .map_err(|error| format!("Could not open wallpaper database: {error}"))?;
    let sql = format!(
        r#"
        SELECT id, source, url_thumb, url_full, photographer, width, height,
               query_used, local_path, is_favorite
        FROM wallpapers
        {clause}
        "#
    );
    let mut statement = connection
        .prepare(&sql)
        .map_err(|error| format!("Could not prepare library query: {error}"))?;
    let rows = statement
        .query_map([], |row| {
            Ok(Wallpaper {
                id: row.get(0)?,
                source: row.get(1)?,
                thumb_url: row.get(2)?,
                full_url: row.get(3)?,
                photographer: row.get(4)?,
                width: row.get(5)?,
                height: row.get(6)?,
                query_used: row.get(7)?,
                local_path: row.get(8)?,
                is_favorite: row.get::<_, i64>(9)? == 1,
            })
        })
        .map_err(|error| format!("Could not query library: {error}"))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("Could not read library rows: {error}"))
}

pub async fn download_wallpaper(
    client: &Client,
    cache_dir: &Path,
    wallpaper: &Wallpaper,
) -> Result<PathBuf, String> {
    let full_dir = cache_dir.join("full");
    fs::create_dir_all(&full_dir)
        .map_err(|error| format!("Could not create wallpaper cache: {error}"))?;
    let target = full_dir.join(format!("{}.jpg", safe_file_stem(&wallpaper.id)));

    if target.exists() {
        return Ok(target);
    }

    let response = client
        .get(&wallpaper.full_url)
        .send()
        .await
        .map_err(|error| format!("Could not download wallpaper: {error}"))?;
    let status = response.status();
    let bytes = response
        .bytes()
        .await
        .map_err(|error| format!("Could not read wallpaper download: {error}"))?;
    if !status.is_success() {
        return Err(format!("Wallpaper download returned {status}"));
    }

    fs::write(&target, bytes).map_err(|error| format!("Could not save wallpaper: {error}"))?;
    Ok(target)
}

pub fn random_cached_wallpaper(db_path: &Path) -> Result<Option<PathBuf>, String> {
    init_database(db_path)?;
    let connection = Connection::open(db_path)
        .map_err(|error| format!("Could not open wallpaper database: {error}"))?;
    let mut statement = connection
        .prepare(
            "SELECT local_path FROM wallpapers WHERE local_path IS NOT NULL ORDER BY RANDOM() LIMIT 1",
        )
        .map_err(|error| format!("Could not prepare cached wallpaper query: {error}"))?;
    let mut rows = statement
        .query([])
        .map_err(|error| format!("Could not query cached wallpaper: {error}"))?;
    if let Some(row) = rows
        .next()
        .map_err(|error| format!("Could not read cached wallpaper: {error}"))?
    {
        let path: String = row
            .get(0)
            .map_err(|error| format!("Could not read cached wallpaper path: {error}"))?;
        Ok(Some(PathBuf::from(path)))
    } else {
        Ok(None)
    }
}

pub fn cache_stats(cache_dir: &Path) -> Result<CacheStats, String> {
    let mut stats = CacheStats { bytes: 0, files: 0 };
    collect_cache_stats(cache_dir, &mut stats)?;
    Ok(stats)
}

fn collect_cache_stats(path: &Path, stats: &mut CacheStats) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(path).map_err(|error| format!("Could not read cache: {error}"))? {
        let entry = entry.map_err(|error| format!("Could not read cache entry: {error}"))?;
        let metadata = entry
            .metadata()
            .map_err(|error| format!("Could not read cache metadata: {error}"))?;
        if metadata.is_dir() {
            collect_cache_stats(&entry.path(), stats)?;
        } else {
            stats.bytes += metadata.len();
            stats.files += 1;
        }
    }
    Ok(())
}

pub fn clear_cache(cache_dir: &Path, db_path: &Path) -> Result<(), String> {
    if cache_dir.exists() {
        fs::remove_dir_all(cache_dir).map_err(|error| format!("Could not clear cache: {error}"))?;
    }
    fs::create_dir_all(cache_dir).map_err(|error| format!("Could not recreate cache: {error}"))?;
    init_database(db_path)?;
    let connection = Connection::open(db_path)
        .map_err(|error| format!("Could not open wallpaper database: {error}"))?;
    connection
        .execute("UPDATE wallpapers SET local_path = NULL", [])
        .map_err(|error| format!("Could not update cache metadata: {error}"))?;
    Ok(())
}

fn safe_file_stem(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '-'
            }
        })
        .collect()
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
    use crate::models::Wallpaper;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_path(name: &str, extension: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("wallpaper-engine-{name}-{nanos}.{extension}"))
    }

    fn sample_wallpaper() -> Wallpaper {
        Wallpaper {
            id: "pexels-42".into(),
            source: "pexels".into(),
            thumb_url: "https://images.pexels.com/thumb.jpg".into(),
            full_url: "https://images.pexels.com/full.jpg".into(),
            photographer: "Photo Person".into(),
            width: 3840,
            height: 2160,
            query_used: Some("forest".into()),
            local_path: None,
            is_favorite: false,
        }
    }

    #[test]
    fn saves_favorite_and_lists_library() {
        let db_path = temp_path("library", "sqlite3");
        init_database(&db_path).expect("database should initialize");

        save_favorite(&db_path, &sample_wallpaper()).expect("favorite should save");
        let library = list_library(&db_path).expect("library should list");

        assert_eq!(library.favorites.len(), 1);
        assert_eq!(library.favorites[0].id, "pexels-42");
        assert!(library.favorites[0].is_favorite);

        let _ = std::fs::remove_file(db_path);
    }

    #[test]
    fn records_downloaded_wallpaper_path() {
        let db_path = temp_path("downloaded", "sqlite3");
        let local_path = temp_path("wallpaper", "jpg");
        init_database(&db_path).expect("database should initialize");

        upsert_downloaded_wallpaper(&db_path, &sample_wallpaper(), &local_path)
            .expect("download should upsert");
        let library = list_library(&db_path).expect("library should list");

        assert_eq!(library.downloaded.len(), 1);
        assert_eq!(
            library.downloaded[0].local_path.as_deref(),
            Some(local_path.to_string_lossy().as_ref())
        );

        let _ = std::fs::remove_file(db_path);
    }
}
