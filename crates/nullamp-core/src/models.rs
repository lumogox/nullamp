use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Track {
    pub id: i64,
    pub file_path: String,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub genre: Option<String>,
    pub track_number: Option<i32>,
    pub duration_secs: Option<f64>,
    pub bpm: Option<i32>,
    pub file_size: Option<i64>,
    #[serde(skip_deserializing)]
    pub file_modified_at: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub text: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub genre: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SyncStatus {
    pub total_tracks: usize,
    pub new_tracks: usize,
    pub removed_tracks: usize,
    pub scan_duration_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScanProgress {
    pub phase: String,
    pub files_found: usize,
    pub files_processed: usize,
    pub files_skipped: usize,
    pub files_failed: usize,
    pub files_removed: usize,
    pub current_file: String,
    pub current_folder: String,
    pub rate: f64,
    pub eta_secs: f64,
    pub elapsed_ms: u64,
}

#[derive(Clone)]
pub struct ScanCancellation(Arc<AtomicBool>);

impl ScanCancellation {
    pub fn new() -> Self {
        Self(Arc::new(AtomicBool::new(false)))
    }
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Relaxed);
    }
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
    pub fn reset(&self) {
        self.0.store(false, Ordering::Relaxed);
    }
}

impl Default for ScanCancellation {
    fn default() -> Self {
        Self::new()
    }
}

/// Intent action parsed from a voice command.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum IntentAction {
    Play,
    Queue,
    Search,
    Pause,
    Resume,
    Next,
    Previous,
    Stop,
    VolumeUp,
    VolumeDown,
    Shuffle,
    ClearQueue,
}

/// Optional query fields for content-based intents.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IntentQuery {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artist: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub album: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub genre: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mood: Option<String>,
    #[serde(rename = "freeText", skip_serializing_if = "Option::is_none")]
    pub free_text: Option<String>,
}

/// A structured music intent extracted from a voice command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MusicIntent {
    pub action: IntentAction,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<IntentQuery>,
}

/// Whisper model catalog entry.
#[derive(Debug, Clone, Serialize)]
pub struct ModelInfo {
    pub id: String,
    pub url: String,
    pub downloaded: bool,
    pub loaded: bool,
}

/// Whisper model download progress.
#[derive(Debug, Clone, Serialize)]
pub struct DownloadProgress {
    pub model: String,
    pub percent: f64,
    pub bytes_downloaded: u64,
    pub total_bytes: u64,
}
