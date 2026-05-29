use crate::models::{CacheStats, ImportResult, Library, Wallpaper, WallpaperPlaylist};
use image::GenericImageView;
use reqwest::header::CONTENT_TYPE;
use reqwest::Client;
use rusqlite::{params, Connection};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::fs;
use std::hash::{Hash, Hasher};
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

            CREATE TABLE IF NOT EXISTS playlists (
                id          TEXT PRIMARY KEY,
                name        TEXT NOT NULL UNIQUE,
                created_at  TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS playlist_wallpapers (
                playlist_id TEXT NOT NULL,
                wallpaper_id TEXT NOT NULL,
                created_at  TEXT NOT NULL,
                PRIMARY KEY (playlist_id, wallpaper_id)
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
            r#"
            DELETE FROM wallpapers
            WHERE id = ?1
              AND is_favorite = 0
              AND local_path IS NULL
              AND used_count = 0
              AND NOT EXISTS (
                SELECT 1 FROM playlist_wallpapers
                WHERE playlist_wallpapers.wallpaper_id = wallpapers.id
              )
            "#,
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
        playlists: list_playlists(db_path)?,
    })
}

pub fn list_playlists(db_path: &Path) -> Result<Vec<WallpaperPlaylist>, String> {
    init_database(db_path)?;
    let connection = Connection::open(db_path)
        .map_err(|error| format!("Could not open wallpaper database: {error}"))?;
    let mut statement = connection
        .prepare("SELECT id, name FROM playlists ORDER BY created_at DESC, name ASC")
        .map_err(|error| format!("Could not prepare playlist query: {error}"))?;
    let playlists = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|error| format!("Could not query playlists: {error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("Could not read playlists: {error}"))?;

    playlists
        .into_iter()
        .map(|(id, name)| {
            Ok(WallpaperPlaylist {
                wallpapers: query_playlist_wallpapers(db_path, &id)?,
                id,
                name,
            })
        })
        .collect()
}

fn query_playlist_wallpapers(db_path: &Path, playlist_id: &str) -> Result<Vec<Wallpaper>, String> {
    let connection = Connection::open(db_path)
        .map_err(|error| format!("Could not open wallpaper database: {error}"))?;
    let mut statement = connection
        .prepare(
            r#"
            SELECT w.id, w.source, w.url_thumb, w.url_full, w.photographer,
                   w.width, w.height, w.query_used, w.mood, w.local_path,
                   w.is_favorite
            FROM playlist_wallpapers pw
            JOIN wallpapers w ON w.id = pw.wallpaper_id
            WHERE pw.playlist_id = ?1
            ORDER BY pw.created_at DESC
            "#,
        )
        .map_err(|error| format!("Could not prepare playlist wallpaper query: {error}"))?;
    let rows = statement
        .query_map(params![playlist_id], |row| {
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
        .map_err(|error| format!("Could not query playlist wallpapers: {error}"))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("Could not read playlist wallpaper rows: {error}"))
}

pub fn create_playlist(db_path: &Path, name: &str) -> Result<Library, String> {
    init_database(db_path)?;
    let name = name.trim();
    if name.is_empty() {
        return Err("Playlist name is required.".into());
    }
    let id = format!(
        "playlist-{}-{}",
        safe_file_stem(name),
        unix_timestamp_string()
    );
    let connection = Connection::open(db_path)
        .map_err(|error| format!("Could not open wallpaper database: {error}"))?;
    connection
        .execute(
            r#"
            INSERT INTO playlists (id, name, created_at)
            VALUES (?1, ?2, ?3)
            ON CONFLICT(name) DO NOTHING
            "#,
            params![id, name, unix_timestamp_string()],
        )
        .map_err(|error| format!("Could not create playlist: {error}"))?;
    list_library(db_path)
}

pub fn delete_playlist(db_path: &Path, playlist_id: &str) -> Result<Library, String> {
    init_database(db_path)?;
    let connection = Connection::open(db_path)
        .map_err(|error| format!("Could not open wallpaper database: {error}"))?;
    connection
        .execute(
            "DELETE FROM playlist_wallpapers WHERE playlist_id = ?1",
            params![playlist_id],
        )
        .map_err(|error| format!("Could not clear playlist wallpapers: {error}"))?;
    connection
        .execute("DELETE FROM playlists WHERE id = ?1", params![playlist_id])
        .map_err(|error| format!("Could not delete playlist: {error}"))?;
    list_library(db_path)
}

pub fn add_wallpaper_to_playlist(
    db_path: &Path,
    playlist_id: &str,
    wallpaper: &Wallpaper,
) -> Result<Library, String> {
    init_database(db_path)?;
    upsert_wallpaper(
        db_path,
        wallpaper,
        wallpaper.local_path.as_deref().map(Path::new),
        false,
        false,
    )?;
    let connection = Connection::open(db_path)
        .map_err(|error| format!("Could not open wallpaper database: {error}"))?;
    let exists: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM playlists WHERE id = ?1",
            params![playlist_id],
            |row| row.get(0),
        )
        .map_err(|error| format!("Could not check playlist: {error}"))?;
    if exists == 0 {
        return Err("Playlist was not found.".into());
    }
    connection
        .execute(
            r#"
            INSERT INTO playlist_wallpapers (playlist_id, wallpaper_id, created_at)
            VALUES (?1, ?2, ?3)
            ON CONFLICT(playlist_id, wallpaper_id) DO NOTHING
            "#,
            params![playlist_id, wallpaper.id, unix_timestamp_string()],
        )
        .map_err(|error| format!("Could not add wallpaper to playlist: {error}"))?;
    list_library(db_path)
}

pub fn remove_wallpaper_from_playlist(
    db_path: &Path,
    playlist_id: &str,
    wallpaper_id: &str,
) -> Result<Library, String> {
    init_database(db_path)?;
    let connection = Connection::open(db_path)
        .map_err(|error| format!("Could not open wallpaper database: {error}"))?;
    connection
        .execute(
            "DELETE FROM playlist_wallpapers WHERE playlist_id = ?1 AND wallpaper_id = ?2",
            params![playlist_id, wallpaper_id],
        )
        .map_err(|error| format!("Could not remove wallpaper from playlist: {error}"))?;
    list_library(db_path)
}

pub fn random_playlist_wallpaper(
    db_path: &Path,
    playlist_id: &str,
) -> Result<Option<Wallpaper>, String> {
    init_database(db_path)?;
    let connection = Connection::open(db_path)
        .map_err(|error| format!("Could not open wallpaper database: {error}"))?;
    let mut statement = connection
        .prepare(
            r#"
            SELECT w.id, w.source, w.url_thumb, w.url_full, w.photographer,
                   w.width, w.height, w.query_used, w.mood, w.local_path,
                   w.is_favorite
            FROM playlist_wallpapers pw
            JOIN wallpapers w ON w.id = pw.wallpaper_id
            WHERE pw.playlist_id = ?1
            ORDER BY RANDOM()
            LIMIT 1
            "#,
        )
        .map_err(|error| format!("Could not prepare playlist random query: {error}"))?;
    let mut rows = statement
        .query(params![playlist_id])
        .map_err(|error| format!("Could not query playlist random wallpaper: {error}"))?;
    if let Some(row) = rows
        .next()
        .map_err(|error| format!("Could not read playlist random wallpaper: {error}"))?
    {
        Ok(Some(Wallpaper {
            id: row
                .get(0)
                .map_err(|error| format!("Could not read wallpaper id: {error}"))?,
            source: row
                .get(1)
                .map_err(|error| format!("Could not read wallpaper source: {error}"))?,
            thumb_url: row
                .get(2)
                .map_err(|error| format!("Could not read wallpaper thumb: {error}"))?,
            full_url: row
                .get(3)
                .map_err(|error| format!("Could not read wallpaper full URL: {error}"))?,
            photographer: row
                .get(4)
                .map_err(|error| format!("Could not read wallpaper photographer: {error}"))?,
            width: row
                .get(5)
                .map_err(|error| format!("Could not read wallpaper width: {error}"))?,
            height: row
                .get(6)
                .map_err(|error| format!("Could not read wallpaper height: {error}"))?,
            query_used: row
                .get(7)
                .map_err(|error| format!("Could not read wallpaper query: {error}"))?,
            mood: row
                .get(8)
                .map_err(|error| format!("Could not read wallpaper mood: {error}"))?,
            local_path: row
                .get(9)
                .map_err(|error| format!("Could not read wallpaper local path: {error}"))?,
            is_favorite: row
                .get::<_, i64>(10)
                .map_err(|error| format!("Could not read wallpaper favorite state: {error}"))?
                == 1,
        }))
    } else {
        Ok(None)
    }
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
        .execute("DELETE FROM playlist_wallpapers", [])
        .map_err(|error| format!("Could not clear playlist wallpapers: {error}"))?;
    connection
        .execute("DELETE FROM playlists", [])
        .map_err(|error| format!("Could not clear playlists: {error}"))?;
    connection
        .execute("DELETE FROM wallpapers", [])
        .map_err(|error| format!("Could not clear library: {error}"))?;
    Ok(())
}

pub fn delete_wallpaper(
    db_path: &Path,
    cache_dir: &Path,
    wallpaper_id: &str,
) -> Result<(), String> {
    init_database(db_path)?;
    let connection = Connection::open(db_path)
        .map_err(|error| format!("Could not open wallpaper database: {error}"))?;
    let local_paths = {
        let mut statement = connection
            .prepare("SELECT local_path FROM wallpapers WHERE id = ?1 AND local_path IS NOT NULL")
            .map_err(|error| format!("Could not prepare wallpaper delete query: {error}"))?;
        let paths = statement
            .query_map(params![wallpaper_id], |row| row.get::<_, String>(0))
            .map_err(|error| format!("Could not query wallpaper delete paths: {error}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("Could not read wallpaper delete paths: {error}"))?;
        paths
    };

    for local_path in local_paths {
        let path = PathBuf::from(local_path);
        if path.exists() {
            fs::remove_file(&path)
                .map_err(|error| format!("Could not delete wallpaper file: {error}"))?;
        }
    }
    delete_matching_cache_files(cache_dir, &safe_file_stem(wallpaper_id))?;

    connection
        .execute(
            "DELETE FROM playlist_wallpapers WHERE wallpaper_id = ?1",
            params![wallpaper_id],
        )
        .map_err(|error| format!("Could not delete playlist wallpaper metadata: {error}"))?;
    connection
        .execute(
            "DELETE FROM wallpapers WHERE id = ?1",
            params![wallpaper_id],
        )
        .map_err(|error| format!("Could not delete wallpaper metadata: {error}"))?;
    Ok(())
}

fn delete_matching_cache_files(cache_dir: &Path, stem: &str) -> Result<(), String> {
    for folder in [cache_dir.join("full"), cache_dir.join("screen")] {
        if !folder.exists() {
            continue;
        }

        for entry in fs::read_dir(&folder)
            .map_err(|error| format!("Could not read wallpaper cache: {error}"))?
        {
            let path = entry
                .map_err(|error| format!("Could not read cached wallpaper entry: {error}"))?
                .path();
            if !path.is_file() {
                continue;
            }

            let Some(file_stem) = path.file_stem().map(|value| value.to_string_lossy()) else {
                continue;
            };
            if file_stem == stem || is_screen_cache_stem_for_wallpaper(&file_stem, stem) {
                fs::remove_file(&path)
                    .map_err(|error| format!("Could not delete cached wallpaper: {error}"))?;
            }
        }
    }
    Ok(())
}

fn is_screen_cache_stem_for_wallpaper(file_stem: &str, wallpaper_stem: &str) -> bool {
    let Some(dimensions) = file_stem.strip_prefix(&format!("{wallpaper_stem}-")) else {
        return false;
    };
    let Some((width, height)) = dimensions.split_once('x') else {
        return false;
    };
    !width.is_empty()
        && !height.is_empty()
        && width.chars().all(|ch| ch.is_ascii_digit())
        && height.chars().all(|ch| ch.is_ascii_digit())
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

pub fn import_local_folder(
    db_path: &Path,
    cache_dir: &Path,
    folder: &Path,
) -> Result<ImportResult, String> {
    if !folder.is_dir() {
        return Err(format!("Folder does not exist: {}", folder.display()));
    }

    init_database(db_path)?;
    let full_dir = cache_dir.join("full");
    fs::create_dir_all(&full_dir)
        .map_err(|error| format!("Could not create wallpaper import cache: {error}"))?;

    let mut image_paths = Vec::new();
    collect_importable_images(folder, &mut image_paths)?;

    let mut result = ImportResult {
        imported: 0,
        skipped: 0,
    };

    for source_path in image_paths {
        match import_local_image(db_path, &full_dir, &source_path) {
            Ok(()) => result.imported += 1,
            Err(_) => result.skipped += 1,
        }
    }

    Ok(result)
}

fn collect_importable_images(folder: &Path, paths: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in
        fs::read_dir(folder).map_err(|error| format!("Could not read import folder: {error}"))?
    {
        let path = entry
            .map_err(|error| format!("Could not read import folder entry: {error}"))?
            .path();
        if path.is_dir() {
            collect_importable_images(&path, paths)?;
        } else if is_supported_image_path(&path) {
            paths.push(path);
        }
    }
    Ok(())
}

fn import_local_image(db_path: &Path, full_dir: &Path, source_path: &Path) -> Result<(), String> {
    let image = image::ImageReader::open(source_path)
        .map_err(|error| format!("Could not open local image: {error}"))?
        .with_guessed_format()
        .map_err(|error| format!("Could not read local image format: {error}"))?
        .decode()
        .map_err(|error| format!("Could not decode local image: {error}"))?;
    let (width, height) = image.dimensions();
    let extension = source_path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase())
        .filter(|value| matches!(value.as_str(), "jpg" | "jpeg" | "png" | "webp"))
        .unwrap_or_else(|| "jpg".into());
    let id = local_wallpaper_id(source_path);
    let target = full_dir.join(format!("{}.{}", safe_file_stem(&id), extension));
    if !target.exists() {
        fs::copy(source_path, &target)
            .map_err(|error| format!("Could not copy local wallpaper into cache: {error}"))?;
    }

    let wallpaper = Wallpaper {
        id,
        source: "local".into(),
        thumb_url: target.to_string_lossy().to_string(),
        full_url: target.to_string_lossy().to_string(),
        photographer: "Local folder".into(),
        width,
        height,
        query_used: Some("local import".into()),
        mood: None,
        local_path: Some(target.to_string_lossy().to_string()),
        is_favorite: false,
    };
    upsert_downloaded_wallpaper(db_path, &wallpaper, &target)
}

fn local_wallpaper_id(path: &Path) -> String {
    let mut hasher = DefaultHasher::new();
    path.to_string_lossy().hash(&mut hasher);
    let stem = path
        .file_stem()
        .map(|value| value.to_string_lossy())
        .unwrap_or_else(|| "wallpaper".into());
    format!("local-{}-{:x}", safe_file_stem(&stem), hasher.finish())
}

fn is_supported_image_path(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .map(|value| {
            matches!(
                value.to_ascii_lowercase().as_str(),
                "jpg" | "jpeg" | "png" | "webp"
            )
        })
        .unwrap_or(false)
}

pub fn last_used_wallpaper(db_path: &Path) -> Result<Option<Wallpaper>, String> {
    init_database(db_path)?;
    let wallpapers = query_wallpapers(
        db_path,
        "WHERE local_path IS NOT NULL ORDER BY last_used DESC, created_at DESC LIMIT 1",
    )?;
    Ok(wallpapers.into_iter().next())
}

pub fn all_wallpapers(db_path: &Path) -> Result<Vec<Wallpaper>, String> {
    init_database(db_path)?;
    query_wallpapers(db_path, "ORDER BY created_at DESC")
}

pub fn restore_wallpaper_metadata(db_path: &Path, wallpapers: &[Wallpaper]) -> Result<(), String> {
    init_database(db_path)?;
    for wallpaper in wallpapers {
        upsert_wallpaper(
            db_path,
            wallpaper,
            wallpaper.local_path.as_deref().map(Path::new),
            wallpaper.is_favorite,
            wallpaper.local_path.is_some(),
        )?;
    }
    Ok(())
}

pub fn restore_playlists(db_path: &Path, playlists: &[WallpaperPlaylist]) -> Result<(), String> {
    init_database(db_path)?;
    let connection = Connection::open(db_path)
        .map_err(|error| format!("Could not open wallpaper database: {error}"))?;
    for playlist in playlists {
        connection
            .execute(
                r#"
                INSERT INTO playlists (id, name, created_at)
                VALUES (?1, ?2, ?3)
                ON CONFLICT(id) DO UPDATE SET name = excluded.name
                "#,
                params![playlist.id, playlist.name, unix_timestamp_string()],
            )
            .map_err(|error| format!("Could not restore playlist: {error}"))?;
        for wallpaper in &playlist.wallpapers {
            upsert_wallpaper(
                db_path,
                wallpaper,
                wallpaper.local_path.as_deref().map(Path::new),
                wallpaper.is_favorite,
                wallpaper.local_path.is_some(),
            )?;
            connection
                .execute(
                    r#"
                    INSERT INTO playlist_wallpapers (playlist_id, wallpaper_id, created_at)
                    VALUES (?1, ?2, ?3)
                    ON CONFLICT(playlist_id, wallpaper_id) DO NOTHING
                    "#,
                    params![playlist.id, wallpaper.id, unix_timestamp_string()],
                )
                .map_err(|error| format!("Could not restore playlist wallpaper: {error}"))?;
        }
    }
    Ok(())
}

pub fn cleanup_old_downloads(
    db_path: &Path,
    cache_dir: &Path,
    days: u64,
    keep_favorites: bool,
) -> Result<u64, String> {
    if days == 0 {
        return Ok(0);
    }

    init_database(db_path)?;
    let cutoff = unix_timestamp_seconds().saturating_sub(days.saturating_mul(86_400));
    let connection = Connection::open(db_path)
        .map_err(|error| format!("Could not open wallpaper database: {error}"))?;
    let sql = if keep_favorites {
        r#"
        SELECT id, local_path
        FROM wallpapers
        WHERE local_path IS NOT NULL
          AND is_favorite = 0
          AND CAST(COALESCE(last_used, created_at, '0') AS INTEGER) < ?1
        "#
    } else {
        r#"
        SELECT id, local_path
        FROM wallpapers
        WHERE local_path IS NOT NULL
          AND CAST(COALESCE(last_used, created_at, '0') AS INTEGER) < ?1
        "#
    };
    let candidates = connection
        .prepare(sql)
        .and_then(|mut statement| {
            statement
                .query_map(params![cutoff.to_string()], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })?
                .collect::<Result<Vec<_>, _>>()
        })
        .map_err(|error| format!("Could not query auto-clean candidates: {error}"))?;

    let mut removed = 0_u64;
    for (id, local_path) in candidates {
        let path = PathBuf::from(local_path);
        if path.exists() {
            fs::remove_file(&path)
                .map_err(|error| format!("Could not delete old wallpaper: {error}"))?;
        }
        delete_matching_cache_files(cache_dir, &safe_file_stem(&id))?;
        connection
            .execute(
                "UPDATE wallpapers SET local_path = NULL WHERE id = ?1",
                params![id],
            )
            .map_err(|error| format!("Could not update auto-clean metadata: {error}"))?;
        removed += 1;
    }

    Ok(removed)
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
    unix_timestamp_seconds().to_string()
}

fn unix_timestamp_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
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
    fn deletes_single_wallpaper_metadata_and_matching_cache_files() {
        let db_path = temp_path("delete-wallpaper", "sqlite3");
        let cache_dir = temp_path("delete-wallpaper-cache-dir", "dir");
        let full_dir = cache_dir.join("full");
        let screen_dir = cache_dir.join("screen");
        std::fs::create_dir_all(&full_dir).expect("full cache dir should exist");
        std::fs::create_dir_all(&screen_dir).expect("screen cache dir should exist");
        let full_path = full_dir.join("pexels-42.jpg");
        let screen_path = screen_dir.join("pexels-42-1920x1080.jpg");
        let other_path = full_dir.join("pexels-43.jpg");
        std::fs::write(&full_path, b"full").expect("full wallpaper should exist");
        std::fs::write(&screen_path, b"screen").expect("screen wallpaper should exist");
        std::fs::write(&other_path, b"other").expect("other wallpaper should exist");
        init_database(&db_path).expect("database should initialize");

        save_favorite(&db_path, &sample_wallpaper()).expect("favorite should save");
        upsert_downloaded_wallpaper(&db_path, &sample_wallpaper(), &screen_path)
            .expect("download should upsert");

        delete_wallpaper(&db_path, &cache_dir, "pexels-42").expect("wallpaper should delete");
        let library = list_library(&db_path).expect("library should list");

        assert!(library.favorites.is_empty());
        assert!(library.downloaded.is_empty());
        assert!(!full_path.exists());
        assert!(!screen_path.exists());
        assert!(other_path.exists());

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

    #[test]
    fn playlist_lifecycle_tracks_wallpapers_and_removals() {
        let db_path = temp_path("playlist", "sqlite3");
        init_database(&db_path).expect("database should initialize");

        let library = create_playlist(&db_path, "Work walls").expect("playlist should create");
        let playlist_id = library.playlists[0].id.clone();
        let library = add_wallpaper_to_playlist(&db_path, &playlist_id, &sample_wallpaper())
            .expect("wallpaper should be added");

        assert_eq!(library.playlists[0].wallpapers.len(), 1);
        assert_eq!(
            random_playlist_wallpaper(&db_path, &playlist_id)
                .expect("random playlist wallpaper should query")
                .map(|wallpaper| wallpaper.id),
            Some("pexels-42".into())
        );

        let library = remove_wallpaper_from_playlist(&db_path, &playlist_id, "pexels-42")
            .expect("wallpaper should be removed");
        assert!(library.playlists[0].wallpapers.is_empty());

        let library = delete_playlist(&db_path, &playlist_id).expect("playlist should delete");
        assert!(library.playlists.is_empty());

        let _ = std::fs::remove_file(db_path);
    }

    #[test]
    fn unfavorite_keeps_wallpaper_metadata_when_it_is_in_playlist() {
        let db_path = temp_path("playlist-unfavorite", "sqlite3");
        init_database(&db_path).expect("database should initialize");
        let wallpaper = sample_wallpaper();
        let library = create_playlist(&db_path, "Keep").expect("playlist should create");
        let playlist_id = library.playlists[0].id.clone();

        save_favorite(&db_path, &wallpaper).expect("favorite should save");
        add_wallpaper_to_playlist(&db_path, &playlist_id, &wallpaper)
            .expect("wallpaper should be added to playlist");
        set_favorite(&db_path, &wallpaper, false).expect("favorite should unset");
        let library = list_library(&db_path).expect("library should list");

        assert!(library.favorites.is_empty());
        assert_eq!(library.playlists[0].wallpapers.len(), 1);
        assert!(!library.playlists[0].wallpapers[0].is_favorite);

        let _ = std::fs::remove_file(db_path);
    }

    #[test]
    fn clear_library_removes_playlists_too() {
        let db_path = temp_path("clear-playlists", "sqlite3");
        let cache_dir = temp_path("clear-playlists-cache", "dir");
        init_database(&db_path).expect("database should initialize");
        let library = create_playlist(&db_path, "Seasonal").expect("playlist should create");
        let playlist_id = library.playlists[0].id.clone();
        add_wallpaper_to_playlist(&db_path, &playlist_id, &sample_wallpaper())
            .expect("wallpaper should be added");

        clear_library(&db_path, &cache_dir).expect("library should clear");
        let library = list_library(&db_path).expect("library should list");

        assert!(library.playlists.is_empty());
        assert!(library.favorites.is_empty());
        assert!(library.downloaded.is_empty());

        let _ = std::fs::remove_file(db_path);
        let _ = std::fs::remove_dir_all(cache_dir);
    }

    #[test]
    fn imports_local_folder_images_into_downloaded_library() {
        let db_path = temp_path("import-local", "sqlite3");
        let cache_dir = temp_path("import-local-cache", "dir");
        let source_dir = temp_path("import-local-source", "dir");
        std::fs::create_dir_all(&source_dir).expect("source dir should exist");
        let source_image = source_dir.join("wall.png");
        image::RgbImage::new(40, 20)
            .save(&source_image)
            .expect("source image should save");

        let result =
            import_local_folder(&db_path, &cache_dir, &source_dir).expect("folder should import");
        let library = list_library(&db_path).expect("library should list");
        let local_path = library.downloaded[0]
            .local_path
            .as_ref()
            .expect("local path should be stored");

        assert_eq!(result.imported, 1);
        assert_eq!(result.skipped, 0);
        assert_eq!(library.downloaded.len(), 1);
        assert_eq!(library.downloaded[0].source, "local");
        assert_eq!(library.downloaded[0].width, 40);
        assert_eq!(library.downloaded[0].height, 20);
        assert!(Path::new(local_path).exists());

        let _ = std::fs::remove_file(db_path);
        let _ = std::fs::remove_dir_all(cache_dir);
        let _ = std::fs::remove_dir_all(source_dir);
    }

    #[test]
    fn auto_clean_removes_old_non_favorite_downloads_only() {
        let db_path = temp_path("auto-clean", "sqlite3");
        let cache_dir = temp_path("auto-clean-cache", "dir");
        let full_dir = cache_dir.join("full");
        std::fs::create_dir_all(&full_dir).expect("cache dir should exist");
        let old_path = full_dir.join("old.jpg");
        let favorite_path = full_dir.join("favorite.jpg");
        std::fs::write(&old_path, b"old").expect("old file should exist");
        std::fs::write(&favorite_path, b"favorite").expect("favorite file should exist");
        init_database(&db_path).expect("database should initialize");

        let mut old = sample_wallpaper();
        old.id = "old".into();
        let mut favorite = sample_wallpaper();
        favorite.id = "favorite".into();
        record_wallpaper_used(&db_path, &old, &old_path).expect("old should record");
        record_wallpaper_used(&db_path, &favorite, &favorite_path).expect("favorite should record");
        save_favorite(&db_path, &favorite).expect("favorite should save");
        let connection = Connection::open(&db_path).expect("database should open");
        connection
            .execute(
                "UPDATE wallpapers SET last_used = '1', created_at = '1'",
                [],
            )
            .expect("timestamps should update");

        let removed =
            cleanup_old_downloads(&db_path, &cache_dir, 1, true).expect("auto-clean should run");
        let library = list_library(&db_path).expect("library should list");

        assert_eq!(removed, 1);
        assert!(!old_path.exists());
        assert!(favorite_path.exists());
        assert_eq!(library.downloaded.len(), 1);
        assert_eq!(library.downloaded[0].id, "favorite");

        let _ = std::fs::remove_file(db_path);
        let _ = std::fs::remove_dir_all(cache_dir);
    }
}
