use crate::models::{CacheStats, Library, Wallpaper};
use reqwest::header::CONTENT_TYPE;
use reqwest::Client;
use rusqlite::{params, Connection};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

static DATABASE_INITIALIZATIONS: OnceLock<Mutex<HashMap<PathBuf, usize>>> = OnceLock::new();

pub fn init_database(db_path: &Path) -> Result<(), String> {
    let key = db_path.to_path_buf();
    let mut initializations = DATABASE_INITIALIZATIONS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .map_err(|error| format!("Could not lock database initialization registry: {error}"))?;

    if initializations.contains_key(&key) && db_path.exists() {
        return Ok(());
    }

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
    *initializations.entry(key).or_insert(0) += 1;
    Ok(())
}

#[cfg(test)]
fn database_initialization_count_for_tests(db_path: &Path) -> usize {
    DATABASE_INITIALIZATIONS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .map(|initializations| {
            initializations
                .get(&db_path.to_path_buf())
                .copied()
                .unwrap_or_default()
        })
        .unwrap_or_default()
}

pub fn save_favorite(db_path: &Path, wallpaper: &Wallpaper) -> Result<(), String> {
    set_favorite(db_path, wallpaper, true)
}

pub fn set_favorite(db_path: &Path, wallpaper: &Wallpaper, favorite: bool) -> Result<(), String> {
    if favorite {
        return upsert_wallpaper(db_path, wallpaper, None, true, false);
    }

    init_database(db_path)?;
    let connection = Connection::open(db_path)
        .map_err(|error| format!("Could not open wallpaper database: {error}"))?;
    connection
        .execute(
            "UPDATE wallpapers SET is_favorite = 0 WHERE id = ?1",
            params![wallpaper.id],
        )
        .map_err(|error| format!("Could not update favorite metadata: {error}"))?;
    connection
        .execute(
            "DELETE FROM wallpapers WHERE id = ?1 AND is_favorite = 0 AND local_path IS NULL AND used_count = 0",
            params![wallpaper.id],
        )
        .map_err(|error| format!("Could not remove unused favorite metadata: {error}"))?;
    Ok(())
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
                local_path, query_used, mood, is_favorite, used_count, last_used, created_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
            ON CONFLICT(id) DO UPDATE SET
                source = excluded.source,
                url_thumb = excluded.url_thumb,
                url_full = excluded.url_full,
                photographer = excluded.photographer,
                width = excluded.width,
                height = excluded.height,
                local_path = COALESCE(excluded.local_path, wallpapers.local_path),
                query_used = COALESCE(excluded.query_used, wallpapers.query_used),
                mood = COALESCE(excluded.mood, wallpapers.mood),
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
                wallpaper.mood,
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

pub fn clear_library(db_path: &Path, cache_dir: &Path) -> Result<(), String> {
    init_database(db_path)?;
    let connection = Connection::open(db_path)
        .map_err(|error| format!("Could not open wallpaper database: {error}"))?;
    let mut statement = connection
        .prepare("SELECT DISTINCT local_path FROM wallpapers WHERE local_path IS NOT NULL")
        .map_err(|error| format!("Could not prepare wallpaper cleanup query: {error}"))?;
    let wallpaper_paths = statement
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|error| format!("Could not query wallpaper cleanup paths: {error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("Could not read wallpaper cleanup paths: {error}"))?;
    drop(statement);

    for wallpaper_path in wallpaper_paths {
        let wallpaper_path = PathBuf::from(wallpaper_path);
        if wallpaper_path.exists() {
            fs::remove_file(&wallpaper_path)
                .map_err(|error| format!("Could not delete wallpaper file: {error}"))?;
        }
    }

    clear_cache_files(cache_dir)?;

    connection
        .execute("DELETE FROM wallpapers", [])
        .map_err(|error| format!("Could not clear library: {error}"))?;
    Ok(())
}

fn query_wallpapers(db_path: &Path, clause: &str) -> Result<Vec<Wallpaper>, String> {
    let connection = Connection::open(db_path)
        .map_err(|error| format!("Could not open wallpaper database: {error}"))?;
    let sql = format!(
        r#"
        SELECT id, source, url_thumb, url_full, photographer, width, height,
               query_used, mood, local_path, is_favorite
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
                mood: row.get(8)?,
                local_path: row.get(9)?,
                is_favorite: row.get::<_, i64>(10)? == 1,
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
    let stem = safe_file_stem(&wallpaper.id);

    if let Some(existing) = existing_cached_download(&full_dir, &stem)? {
        return Ok(existing);
    }

    let response = client
        .get(&wallpaper.full_url)
        .send()
        .await
        .map_err(|error| format!("Could not download wallpaper: {error}"))?;
    let status = response.status();
    let content_type = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    let bytes = response
        .bytes()
        .await
        .map_err(|error| format!("Could not read wallpaper download: {error}"))?;
    if !status.is_success() {
        return Err(format!("Wallpaper download returned {status}"));
    }

    let extension = extension_from_url(&wallpaper.full_url)
        .or_else(|| {
            content_type
                .as_deref()
                .and_then(extension_from_content_type)
        })
        .unwrap_or("jpg");
    let target = full_dir.join(format!("{stem}.{extension}"));
    fs::write(&target, bytes).map_err(|error| format!("Could not save wallpaper: {error}"))?;
    Ok(target)
}

fn existing_cached_download(full_dir: &Path, stem: &str) -> Result<Option<PathBuf>, String> {
    if !full_dir.exists() {
        return Ok(None);
    }

    for entry in fs::read_dir(full_dir)
        .map_err(|error| format!("Could not read wallpaper cache: {error}"))?
    {
        let path = entry
            .map_err(|error| format!("Could not read cached wallpaper entry: {error}"))?
            .path();
        if path
            .file_stem()
            .is_some_and(|value| value.to_string_lossy() == stem)
        {
            return Ok(Some(path));
        }
    }

    Ok(None)
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
    clear_cache_files(cache_dir)?;
    init_database(db_path)?;
    let connection = Connection::open(db_path)
        .map_err(|error| format!("Could not open wallpaper database: {error}"))?;
    connection
        .execute("UPDATE wallpapers SET local_path = NULL", [])
        .map_err(|error| format!("Could not update cache metadata: {error}"))?;
    Ok(())
}

fn clear_cache_files(cache_dir: &Path) -> Result<(), String> {
    if cache_dir.exists() {
        fs::remove_dir_all(cache_dir).map_err(|error| format!("Could not clear cache: {error}"))?;
    }
    fs::create_dir_all(cache_dir).map_err(|error| format!("Could not recreate cache: {error}"))?;
    Ok(())
}

pub fn enforce_cache_limit_mb(
    cache_dir: &Path,
    db_path: &Path,
    limit_mb: u64,
) -> Result<(), String> {
    enforce_cache_limit_bytes(cache_dir, db_path, limit_mb.saturating_mul(1024 * 1024))
}

pub fn enforce_cache_limit_bytes(
    cache_dir: &Path,
    db_path: &Path,
    limit_bytes: u64,
) -> Result<(), String> {
    if limit_bytes == 0 || !cache_dir.exists() {
        return Ok(());
    }

    init_database(db_path)?;
    let connection = Connection::open(db_path)
        .map_err(|error| format!("Could not open wallpaper database: {error}"))?;
    let mut statement = connection
        .prepare(
            r#"
            SELECT id, local_path
            FROM wallpapers
            WHERE local_path IS NOT NULL
            ORDER BY COALESCE(last_used, created_at), created_at
            "#,
        )
        .map_err(|error| format!("Could not prepare cache eviction query: {error}"))?;
    let candidates = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|error| format!("Could not query cache eviction candidates: {error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("Could not read cache eviction candidates: {error}"))?;

    for (id, local_path) in candidates {
        if cache_stats(cache_dir)?.bytes <= limit_bytes {
            return Ok(());
        }

        let path = PathBuf::from(local_path);
        if path.exists() {
            fs::remove_file(&path)
                .map_err(|error| format!("Could not evict cached wallpaper: {error}"))?;
        }
        connection
            .execute(
                "UPDATE wallpapers SET local_path = NULL WHERE id = ?1",
                params![id],
            )
            .map_err(|error| format!("Could not update evicted wallpaper metadata: {error}"))?;
    }

    for path in cache_files_by_modified_time(cache_dir)? {
        if cache_stats(cache_dir)?.bytes <= limit_bytes {
            break;
        }
        if path.is_file() {
            let _ = fs::remove_file(path);
        }
    }

    Ok(())
}

fn cache_files_by_modified_time(cache_dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    collect_cache_files(cache_dir, &mut files)?;
    files.sort_by_key(|path| {
        path.metadata()
            .and_then(|metadata| metadata.modified())
            .unwrap_or(UNIX_EPOCH)
    });
    Ok(files)
}

fn collect_cache_files(path: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(path).map_err(|error| format!("Could not read cache: {error}"))? {
        let path = entry
            .map_err(|error| format!("Could not read cache entry: {error}"))?
            .path();
        if path.is_dir() {
            collect_cache_files(&path, files)?;
        } else {
            files.push(path);
        }
    }
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

fn extension_from_url(value: &str) -> Option<&'static str> {
    let path = value.split('?').next().unwrap_or(value);
    let extension = path.rsplit('.').next()?.to_ascii_lowercase();
    match extension.as_str() {
        "jpg" | "jpeg" => Some("jpg"),
        "png" => Some("png"),
        "webp" => Some("webp"),
        _ => None,
    }
}

fn extension_from_content_type(value: &str) -> Option<&'static str> {
    match value
        .split(';')
        .next()?
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "image/jpeg" => Some("jpg"),
        "image/png" => Some("png"),
        "image/webp" => Some("webp"),
        _ => None,
    }
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
            mood: Some("nature".into()),
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
        assert_eq!(library.downloaded[0].mood.as_deref(), Some("nature"));

        let _ = std::fs::remove_file(db_path);
    }

    #[test]
    fn init_database_tracks_schema_initialization_once_per_path() {
        let db_path = temp_path("init-once", "sqlite3");

        init_database(&db_path).expect("database should initialize");
        init_database(&db_path).expect("database should remain initialized");

        assert_eq!(database_initialization_count_for_tests(&db_path), 1);

        let _ = std::fs::remove_file(db_path);
    }

    #[test]
    fn enforce_cache_limit_evicts_least_recently_used_downloads() {
        let db_path = temp_path("evict", "sqlite3");
        let cache_dir = temp_path("cache-dir", "dir");
        let full_dir = cache_dir.join("full");
        std::fs::create_dir_all(&full_dir).expect("cache dir should be created");
        let old_path = full_dir.join("old.jpg");
        let new_path = full_dir.join("new.jpg");
        std::fs::write(&old_path, vec![1_u8; 700_000]).expect("old file should save");
        std::fs::write(&new_path, vec![2_u8; 700_000]).expect("new file should save");

        let mut old = sample_wallpaper();
        old.id = "old".into();
        let mut new = sample_wallpaper();
        new.id = "new".into();
        init_database(&db_path).expect("database should initialize");
        record_wallpaper_used(&db_path, &old, &old_path).expect("old should record");
        record_wallpaper_used(&db_path, &new, &new_path).expect("new should record");
        let connection = Connection::open(&db_path).expect("database should open");
        connection
            .execute(
                "UPDATE wallpapers SET last_used = ?1 WHERE id = ?2",
                params!["1", "old"],
            )
            .expect("old timestamp should update");
        connection
            .execute(
                "UPDATE wallpapers SET last_used = ?1 WHERE id = ?2",
                params!["2", "new"],
            )
            .expect("new timestamp should update");

        enforce_cache_limit_bytes(&cache_dir, &db_path, 1_000_000)
            .expect("cache limit should be enforced");
        let library = list_library(&db_path).expect("library should list");

        assert!(!old_path.exists());
        assert!(new_path.exists());
        assert_eq!(library.downloaded.len(), 1);
        assert_eq!(library.downloaded[0].id, "new");

        let _ = std::fs::remove_file(db_path);
        let _ = std::fs::remove_dir_all(cache_dir);
    }

    #[test]
    fn clears_library_metadata_and_wallpaper_files() {
        let db_path = temp_path("clear-library", "sqlite3");
        let cache_dir = temp_path("clear-library-cache-dir", "dir");
        let local_path = temp_path("clear-library-wallpaper", "jpg");
        std::fs::write(&local_path, b"wallpaper").expect("wallpaper file should exist");
        init_database(&db_path).expect("database should initialize");

        save_favorite(&db_path, &sample_wallpaper()).expect("favorite should save");
        upsert_downloaded_wallpaper(&db_path, &sample_wallpaper(), &local_path)
            .expect("download should upsert");

        clear_library(&db_path, &cache_dir).expect("library should clear");
        let library = list_library(&db_path).expect("library should list");

        assert!(library.favorites.is_empty());
        assert!(library.downloaded.is_empty());
        assert!(!local_path.exists());

        let _ = std::fs::remove_file(db_path);
        let _ = std::fs::remove_dir_all(cache_dir);
    }

    #[test]
    fn clear_library_removes_entire_wallpaper_cache_directory() {
        let db_path = temp_path("clear-library-cache", "sqlite3");
        let cache_dir = temp_path("clear-library-cache-dir", "dir");
        let full_dir = cache_dir.join("full");
        let screen_dir = cache_dir.join("screen");
        std::fs::create_dir_all(&full_dir).expect("full cache dir should exist");
        std::fs::create_dir_all(&screen_dir).expect("screen cache dir should exist");
        std::fs::write(full_dir.join("orphan.jpg"), b"orphan")
            .expect("orphan download should exist");
        std::fs::write(screen_dir.join("prepared.jpg"), b"prepared")
            .expect("prepared wallpaper should exist");
        init_database(&db_path).expect("database should initialize");

        clear_library(&db_path, &cache_dir).expect("library should clear");

        assert!(cache_dir.exists());
        assert_eq!(
            cache_stats(&cache_dir)
                .expect("cache stats should load")
                .files,
            0
        );

        let _ = std::fs::remove_file(db_path);
        let _ = std::fs::remove_dir_all(cache_dir);
    }

    #[test]
    fn removes_favorite_without_removing_downloaded_wallpaper() {
        let db_path = temp_path("unfavorite", "sqlite3");
        let local_path = temp_path("unfavorite-wallpaper", "jpg");
        std::fs::write(&local_path, b"wallpaper").expect("wallpaper file should exist");
        init_database(&db_path).expect("database should initialize");

        let wallpaper = sample_wallpaper();
        save_favorite(&db_path, &wallpaper).expect("favorite should save");
        upsert_downloaded_wallpaper(&db_path, &wallpaper, &local_path)
            .expect("download should upsert");

        set_favorite(&db_path, &wallpaper, false).expect("favorite should unset");
        let library = list_library(&db_path).expect("library should list");

        assert!(library.favorites.is_empty());
        assert_eq!(library.downloaded.len(), 1);
        assert!(!library.downloaded[0].is_favorite);
        assert!(local_path.exists());

        let _ = std::fs::remove_file(db_path);
        let _ = std::fs::remove_file(local_path);
    }
}
