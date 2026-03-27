use crate::db;
use crate::error::AppError;
use crate::models::{ScanCancellation, ScanProgress, SyncStatus, Track};
use lofty::prelude::*;
use lofty::probe::Probe;
use rayon::prelude::*;
use rusqlite::Connection;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Instant;
use walkdir::WalkDir;

const AUDIO_EXTENSIONS: &[&str] = &[
    "mp3", "flac", "ogg", "m4a", "wav", "aiff", "wma", "opus", "ape", "wv",
];

const BATCH_SIZE: usize = 100;

/// Trait for emitting scan progress — decouples the indexer from any UI framework.
/// Tauri implements this via AppHandle::emit(), Iced via iced::Subscription channels, etc.
pub trait ProgressEmitter: Send + Sync {
    fn emit(&self, progress: &ScanProgress);
}

/// No-op emitter for testing or headless usage.
pub struct NullEmitter;
impl ProgressEmitter for NullEmitter {
    fn emit(&self, _progress: &ScanProgress) {}
}

/// Discover all supported audio files in a directory tree.
fn discover_files(root: &Path, cancel: &ScanCancellation) -> Vec<PathBuf> {
    WalkDir::new(root)
        .follow_links(true)
        .into_iter()
        .filter_map(|entry| {
            if cancel.is_cancelled() {
                return None;
            }
            entry.ok()
        })
        .filter(|entry| {
            entry.file_type().is_file()
                && entry
                    .path()
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| AUDIO_EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()))
                    .unwrap_or(false)
        })
        .map(|entry| entry.into_path())
        .collect()
}

/// Extract metadata from an audio file using lofty.
pub fn extract_metadata(path: &Path) -> Track {
    let file_path = path.to_string_lossy().to_string();
    let file_meta = std::fs::metadata(path).ok();
    let file_size = file_meta.as_ref().map(|m| m.len() as i64);
    let file_modified_at = file_meta
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64);

    let mut track = Track {
        id: 0,
        file_path,
        title: None,
        artist: None,
        album: None,
        genre: None,
        track_number: None,
        duration_secs: None,
        bpm: None,
        file_size,
        file_modified_at,
    };

    let tagged_file = match Probe::open(path).and_then(|p| p.read()) {
        Ok(f) => f,
        Err(e) => {
            log::warn!("Failed to read tags from {}: {e}", path.display());
            track.title = path.file_stem().map(|s| s.to_string_lossy().to_string());
            return track;
        }
    };

    let properties = tagged_file.properties();
    track.duration_secs = Some(properties.duration().as_secs_f64());

    let tag = tagged_file
        .primary_tag()
        .or_else(|| tagged_file.first_tag());

    if let Some(tag) = tag {
        track.title = tag.title().map(|s| s.to_string());
        track.artist = tag.artist().map(|s| s.to_string());
        track.album = tag.album().map(|s| s.to_string());
        track.genre = tag.genre().map(|s| s.to_string());
        track.track_number = tag.track().map(|t| t as i32);
    }

    if track.title.is_none() && track.artist.is_none() {
        if let Some(stem) = path.file_stem().map(|s| s.to_string_lossy().to_string()) {
            let (artist, title) = parse_filename(&stem);
            track.artist = artist;
            track.title = Some(title);
        }
    } else if track.title.is_none() {
        track.title = path.file_stem().map(|s| s.to_string_lossy().to_string());
    }

    track
}

/// Parallel sync: discover -> filter -> rayon extract -> batch DB insert -> remove missing.
pub fn sync_library_parallel(
    conn: &Arc<Mutex<Connection>>,
    music_dir: &Path,
    emitter: &dyn ProgressEmitter,
    cancel: &ScanCancellation,
) -> Result<SyncStatus, AppError> {
    let start = Instant::now();
    cancel.reset();

    // -- Phase 1: Discovery --
    log::info!("Scanning directory: {}", music_dir.display());
    emitter.emit(&ScanProgress {
        phase: "discovering".into(),
        files_found: 0,
        files_processed: 0,
        files_skipped: 0,
        files_failed: 0,
        files_removed: 0,
        current_folder: music_dir.to_string_lossy().to_string(),
        rate: 0.0,
        eta_secs: 0.0,
        elapsed_ms: 0,
    });

    let all_files = discover_files(music_dir, cancel);
    let total_found = all_files.len();
    log::info!("Found {} audio files", total_found);

    if cancel.is_cancelled() {
        return Err(AppError::Other("Scan cancelled".into()));
    }

    // -- Phase 2: Filter to new files only (brief mutex lock) --
    let existing_paths: HashSet<String> = {
        let db = conn.lock().map_err(|e| AppError::Other(e.to_string()))?;
        db::get_all_file_paths(&db)?
    };

    let new_files: Vec<PathBuf> = all_files
        .iter()
        .filter(|p| !existing_paths.contains(&p.to_string_lossy().to_string()))
        .cloned()
        .collect();

    let skipped = total_found - new_files.len();
    let to_process = new_files.len();
    log::info!(
        "{} new files to index, {} already in DB",
        to_process,
        skipped
    );

    if cancel.is_cancelled() {
        return Err(AppError::Other("Scan cancelled".into()));
    }

    // -- Phase 3: Parallel extract + batched DB insert --
    let processed = Arc::new(AtomicUsize::new(0));
    let failed = Arc::new(AtomicUsize::new(0));
    let (tx, rx) = mpsc::channel::<Track>();

    // Clone Arcs for the DB writer thread
    let conn_writer = Arc::clone(conn);
    let cancel_writer = cancel.clone();

    let writer_handle = std::thread::spawn(move || -> Result<usize, AppError> {
        let mut batch: Vec<Track> = Vec::with_capacity(BATCH_SIZE);
        let mut inserted = 0usize;

        for track in rx.iter() {
            if cancel_writer.is_cancelled() {
                break;
            }

            batch.push(track);

            if batch.len() >= BATCH_SIZE {
                let db = conn_writer
                    .lock()
                    .map_err(|e| AppError::Other(e.to_string()))?;
                inserted += db::insert_tracks_batch(&db, &batch)?;
                drop(db);
                batch.clear();
            }
        }

        // Flush remaining batch
        if !batch.is_empty() {
            let db = conn_writer
                .lock()
                .map_err(|e| AppError::Other(e.to_string()))?;
            inserted += db::insert_tracks_batch(&db, &batch)?;
        }

        Ok(inserted)
    });

    // Parallel metadata extraction with rayon
    new_files.par_iter().for_each(|path| {
        if cancel.is_cancelled() {
            return;
        }
        let track = extract_metadata(path);
        processed.fetch_add(1, Ordering::Relaxed);
        if tx.send(track).is_err() {
            failed.fetch_add(1, Ordering::Relaxed);
        }
    });
    drop(tx); // Close channel so writer thread finishes

    let new_tracks = writer_handle
        .join()
        .map_err(|_| AppError::Other("Writer thread panicked".into()))??;

    if cancel.is_cancelled() {
        let total_tracks = {
            let db = conn.lock().map_err(|e| AppError::Other(e.to_string()))?;
            db::get_track_count(&db).unwrap_or(0)
        };
        emitter.emit(&ScanProgress {
            phase: "cancelled".into(),
            files_found: total_found,
            files_processed: processed.load(Ordering::Relaxed),
            files_skipped: skipped,
            files_failed: failed.load(Ordering::Relaxed),
            files_removed: 0,
            current_folder: String::new(),
            rate: 0.0,
            eta_secs: 0.0,
            elapsed_ms: start.elapsed().as_millis() as u64,
        });
        return Ok(SyncStatus {
            total_tracks,
            new_tracks,
            removed_tracks: 0,
            scan_duration_ms: start.elapsed().as_millis() as u64,
        });
    }

    // Emit indexing progress one final time before removal phase
    let elapsed = start.elapsed().as_secs_f64();
    let rate = if elapsed > 0.0 {
        to_process as f64 / elapsed
    } else {
        0.0
    };
    emitter.emit(&ScanProgress {
        phase: "indexing".into(),
        files_found: total_found,
        files_processed: to_process,
        files_skipped: skipped,
        files_failed: failed.load(Ordering::Relaxed),
        files_removed: 0,
        current_folder: String::new(),
        rate,
        eta_secs: 0.0,
        elapsed_ms: start.elapsed().as_millis() as u64,
    });

    // -- Phase 4: Remove missing tracks (HashSet O(n)) --
    emitter.emit(&ScanProgress {
        phase: "removing".into(),
        files_found: total_found,
        files_processed: to_process,
        files_skipped: skipped,
        files_failed: failed.load(Ordering::Relaxed),
        files_removed: 0,
        current_folder: String::new(),
        rate: 0.0,
        eta_secs: 0.0,
        elapsed_ms: start.elapsed().as_millis() as u64,
    });

    let disk_paths: HashSet<String> = all_files
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect();

    let removed_tracks = {
        let db = conn.lock().map_err(|e| AppError::Other(e.to_string()))?;
        db::remove_missing_tracks(&db, &disk_paths)?
    };

    let total_tracks = {
        let db = conn.lock().map_err(|e| AppError::Other(e.to_string()))?;
        db::get_track_count(&db).unwrap_or(0)
    };

    let scan_duration_ms = start.elapsed().as_millis() as u64;

    log::info!(
        "Sync complete: {} total, {} new, {} removed, {} skipped, {}ms",
        total_tracks,
        new_tracks,
        removed_tracks,
        skipped,
        scan_duration_ms
    );

    // Final progress event
    emitter.emit(&ScanProgress {
        phase: "complete".into(),
        files_found: total_found,
        files_processed: to_process,
        files_skipped: skipped,
        files_failed: failed.load(Ordering::Relaxed),
        files_removed: removed_tracks,
        current_folder: String::new(),
        rate: 0.0,
        eta_secs: 0.0,
        elapsed_ms: scan_duration_ms,
    });

    Ok(SyncStatus {
        total_tracks,
        new_tracks,
        removed_tracks,
        scan_duration_ms,
    })
}

/// Parse artist and title from a filename when tags are missing.
fn parse_filename(stem: &str) -> (Option<String>, String) {
    if let Some((artist, title)) = stem.split_once(" - ") {
        return (Some(humanize(artist.trim())), humanize(title.trim()));
    }

    if let Some((first, rest)) = stem.split_once('-') {
        let first = first.trim();
        let rest = rest.trim();
        let title_part = strip_trailing_id(rest);
        if !first.is_empty() && !title_part.is_empty() {
            return (Some(humanize(first)), humanize(&title_part));
        }
    }

    (None, humanize(stem))
}

fn strip_trailing_id(s: &str) -> String {
    if let Some(last_dash) = s.rfind('-') {
        let suffix = &s[last_dash + 1..];
        if suffix.chars().all(|c| c.is_ascii_digit()) && suffix.len() >= 3 {
            return s[..last_dash].to_string();
        }
    }
    s.to_string()
}

fn humanize(s: &str) -> String {
    s.replace('_', " ")
        .replace('-', " ")
        .split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => {
                    let upper: String = c.to_uppercase().collect();
                    upper + &chars.as_str().to_lowercase()
                }
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
