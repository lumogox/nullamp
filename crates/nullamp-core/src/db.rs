use crate::error::AppError;
use crate::models::{SearchQuery, Track};
use rusqlite::{params, Connection};
use std::collections::HashSet;
use std::path::Path;
use std::sync::{Arc, Mutex};

pub type DbState = Arc<Mutex<Connection>>;

pub fn init_db(db_path: &Path) -> Result<Connection, AppError> {
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let conn = Connection::open(db_path)?;

    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS tracks (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            file_path TEXT NOT NULL UNIQUE,
            title TEXT,
            artist TEXT,
            album TEXT,
            genre TEXT,
            track_number INTEGER,
            duration_secs REAL,
            bpm INTEGER,
            file_size INTEGER,
            file_modified_at INTEGER,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP
        );
        CREATE INDEX IF NOT EXISTS idx_tracks_artist ON tracks(artist);
        CREATE INDEX IF NOT EXISTS idx_tracks_album ON tracks(album);
        CREATE INDEX IF NOT EXISTS idx_tracks_genre ON tracks(genre);
        CREATE INDEX IF NOT EXISTS idx_tracks_title ON tracks(title);
        ",
    )?;

    // Migration: add file_modified_at for existing databases
    let has_column: bool = conn
        .prepare("PRAGMA table_info(tracks)")?
        .query_map([], |row| row.get::<_, String>(1))?
        .filter_map(|r| r.ok())
        .any(|name| name == "file_modified_at");
    if !has_column {
        conn.execute_batch("ALTER TABLE tracks ADD COLUMN file_modified_at INTEGER")?;
        log::info!("Migrated: added file_modified_at column");
    }

    log::info!("Database initialized at: {}", db_path.display());
    Ok(conn)
}

pub fn insert_track(conn: &Connection, track: &Track) -> Result<(), AppError> {
    conn.execute(
        "INSERT OR REPLACE INTO tracks (file_path, title, artist, album, genre, track_number, duration_secs, bpm, file_size, file_modified_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            track.file_path,
            track.title,
            track.artist,
            track.album,
            track.genre,
            track.track_number,
            track.duration_secs,
            track.bpm,
            track.file_size,
            track.file_modified_at,
        ],
    )?;
    Ok(())
}

/// Insert multiple tracks in a single transaction (batch insert).
pub fn insert_tracks_batch(conn: &Connection, tracks: &[Track]) -> Result<usize, AppError> {
    if tracks.is_empty() {
        return Ok(0);
    }
    conn.execute_batch("BEGIN")?;
    let mut count = 0;
    for track in tracks {
        conn.execute(
            "INSERT OR REPLACE INTO tracks (file_path, title, artist, album, genre, track_number, duration_secs, bpm, file_size, file_modified_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                track.file_path,
                track.title,
                track.artist,
                track.album,
                track.genre,
                track.track_number,
                track.duration_secs,
                track.bpm,
                track.file_size,
                track.file_modified_at,
            ],
        )?;
        count += 1;
    }
    conn.execute_batch("COMMIT")?;
    Ok(count)
}

/// Get all file paths currently in the DB as a HashSet for O(1) lookups.
pub fn get_all_file_paths(conn: &Connection) -> Result<HashSet<String>, AppError> {
    let mut stmt = conn.prepare("SELECT file_path FROM tracks")?;
    let paths = stmt
        .query_map([], |row| row.get::<_, String>(0))?
        .filter_map(|r| r.ok())
        .collect();
    Ok(paths)
}

/// Remove tracks whose file_path is NOT in the provided HashSet. O(n) algorithm.
pub fn remove_missing_tracks(
    conn: &Connection,
    disk_paths: &HashSet<String>,
) -> Result<usize, AppError> {
    let mut stmt = conn.prepare("SELECT file_path FROM tracks")?;
    let to_remove: Vec<String> = stmt
        .query_map([], |row| row.get::<_, String>(0))?
        .filter_map(|r| r.ok())
        .filter(|p| !disk_paths.contains(p))
        .collect();

    if to_remove.is_empty() {
        return Ok(0);
    }

    conn.execute_batch("BEGIN")?;
    let mut del_stmt = conn.prepare_cached("DELETE FROM tracks WHERE file_path = ?1")?;
    for path in &to_remove {
        del_stmt.execute(params![path])?;
    }
    conn.execute_batch("COMMIT")?;
    Ok(to_remove.len())
}

pub fn get_track_count(conn: &Connection) -> Result<usize, AppError> {
    Ok(conn.query_row("SELECT COUNT(*) FROM tracks", [], |row| row.get(0))?)
}

pub fn get_all_artists(conn: &Connection) -> Result<Vec<String>, AppError> {
    let mut stmt = conn
        .prepare("SELECT DISTINCT artist FROM tracks WHERE artist IS NOT NULL ORDER BY artist")?;
    let artists = stmt
        .query_map([], |row| row.get(0))?
        .filter_map(|r| r.ok())
        .collect();
    Ok(artists)
}

pub fn get_albums_by_artist(conn: &Connection, artist: &str) -> Result<Vec<String>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT DISTINCT album FROM tracks WHERE artist = ?1 AND album IS NOT NULL ORDER BY album",
    )?;
    let albums = stmt
        .query_map(params![artist], |row| row.get(0))?
        .filter_map(|r| r.ok())
        .collect();
    Ok(albums)
}

pub fn get_tracks(
    conn: &Connection,
    artist: Option<&str>,
    album: Option<&str>,
) -> Result<Vec<Track>, AppError> {
    let sql = match (artist, album) {
        (Some(_), Some(_)) => {
            "SELECT id, file_path, title, artist, album, genre, track_number, duration_secs, bpm, file_size
             FROM tracks WHERE artist = ?1 AND album = ?2 ORDER BY track_number, title"
        }
        (Some(_), None) => {
            "SELECT id, file_path, title, artist, album, genre, track_number, duration_secs, bpm, file_size
             FROM tracks WHERE artist = ?1 ORDER BY album, track_number, title"
        }
        _ => {
            "SELECT id, file_path, title, artist, album, genre, track_number, duration_secs, bpm, file_size
             FROM tracks ORDER BY artist, album, track_number, title"
        }
    };

    let mut stmt = conn.prepare(sql)?;

    let rows = match (artist, album) {
        (Some(a), Some(al)) => stmt.query_map(params![a, al], row_to_track)?,
        (Some(a), None) => stmt.query_map(params![a], row_to_track)?,
        _ => stmt.query_map([], row_to_track)?,
    };

    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn search_tracks(conn: &Connection, query: &SearchQuery) -> Result<Vec<Track>, AppError> {
    let mut conditions = Vec::new();
    let mut param_values: Vec<String> = Vec::new();

    if let Some(ref artist) = query.artist {
        conditions.push(format!("artist LIKE ?{}", param_values.len() + 1));
        param_values.push(format!("%{artist}%"));
    }
    if let Some(ref album) = query.album {
        conditions.push(format!("album LIKE ?{}", param_values.len() + 1));
        param_values.push(format!("%{album}%"));
    }
    if let Some(ref genre) = query.genre {
        conditions.push(format!("genre LIKE ?{}", param_values.len() + 1));
        param_values.push(format!("%{genre}%"));
    }
    if let Some(ref text) = query.text {
        let idx = param_values.len() + 1;
        conditions.push(format!(
            "(title LIKE ?{idx} OR artist LIKE ?{idx} OR album LIKE ?{idx} OR genre LIKE ?{idx})"
        ));
        param_values.push(format!("%{text}%"));
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    let sql = format!(
        "SELECT id, file_path, title, artist, album, genre, track_number, duration_secs, bpm, file_size
         FROM tracks {where_clause}
         ORDER BY artist, album, track_number, title
         LIMIT 100"
    );

    let mut stmt = conn.prepare(&sql)?;
    let params: Vec<&dyn rusqlite::types::ToSql> = param_values
        .iter()
        .map(|v| v as &dyn rusqlite::types::ToSql)
        .collect();

    let rows = stmt.query_map(params.as_slice(), row_to_track)?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

fn row_to_track(row: &rusqlite::Row) -> Result<Track, rusqlite::Error> {
    Ok(Track {
        id: row.get(0)?,
        file_path: row.get(1)?,
        title: row.get(2)?,
        artist: row.get(3)?,
        album: row.get(4)?,
        genre: row.get(5)?,
        track_number: row.get(6)?,
        duration_secs: row.get(7)?,
        bpm: row.get(8)?,
        file_size: row.get(9)?,
        file_modified_at: None,
    })
}
