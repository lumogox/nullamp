use crate::message::{
    LibraryViewMode, Message, RepeatMode, Tab, TrackAction, TrackSource, WhisperModelInfo,
};
use crate::theme;
use crate::views;

use iced::widget::{column, container, stack};
use iced::{Element, Length, Point, Subscription, Task, Theme};

use nullamp_audio::capture::MicCapture;
use nullamp_audio::player::{AudioPlayer, PlayState};
use nullamp_core::db::{self, DbState};
use nullamp_core::models::{IntentAction, ScanCancellation, SearchQuery, Track};
use nullamp_core::settings::Settings;
use nullamp_core::whisper::WhisperState;

use futures::StreamExt as _;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// All 4 supported Whisper model sizes: (id, filename, size_mb).
const WHISPER_MODELS: &[(&str, &str, u32)] = &[
    ("tiny", "ggml-tiny.bin", 75),
    ("base", "ggml-base.bin", 142),
    ("small", "ggml-small.bin", 466),
    ("medium", "ggml-medium.bin", 1500),
];

/// Global keyboard handler — must be a fn pointer (no captures).
/// The input_focused guard is applied in update() per-message.
fn keyboard_handler(
    key: iced::keyboard::Key,
    modifiers: iced::keyboard::Modifiers,
) -> Option<Message> {
    use iced::keyboard::key::Named;
    use iced::keyboard::Key;
    match key {
        Key::Named(Named::Space) => Some(Message::TogglePlayPause),
        Key::Named(Named::ArrowRight) if modifiers.command() => Some(Message::NextTrack),
        Key::Named(Named::ArrowLeft) if modifiers.command() => Some(Message::PrevTrack),
        Key::Named(Named::ArrowRight) => Some(Message::SeekDelta(5.0)),
        Key::Named(Named::ArrowLeft) => Some(Message::SeekDelta(-5.0)),
        Key::Named(Named::ArrowUp) if modifiers.command() => Some(Message::VolumeChanged(90.0)),
        Key::Named(Named::ArrowDown) if modifiers.command() => Some(Message::VolumeChanged(70.0)),
        Key::Character(ref c) => match c.as_str() {
            "n" | "N" => Some(Message::NextTrack),
            "p" | "P" => Some(Message::PrevTrack),
            _ => None,
        },
        Key::Named(Named::Escape) => Some(Message::CloseSettings),
        _ => None,
    }
}

/// State for a currently-open context menu.
#[derive(Debug, Clone)]
pub struct ContextMenuState {
    pub source: TrackSource,
    pub track_index: usize,
    /// Cursor position captured at right-click — frozen so the menu doesn't follow the cursor.
    pub position: Point,
}

/// Top-level application state.
pub struct Nullamp {
    // Core
    pub db: DbState,
    pub settings: Settings,
    settings_path: std::path::PathBuf,

    // Audio
    pub player: AudioPlayer,

    // Playlist / queue — single source of truth for playback AND the queue panel.
    // Clicking a library track replaces it with the full library so Prev/Next still work.
    // Right-click → "Add to Queue" appends without replacing.
    pub playlist: Vec<Track>,
    pub current_index: Option<usize>,
    pub shuffle: bool,
    pub repeat: RepeatMode,

    // Library
    pub all_tracks: Vec<Track>,
    pub artists: Vec<String>,
    pub search_query: String,
    pub search_results: Vec<Track>,
    pub library_view_mode: LibraryViewMode,
    pub expanded_artists: HashSet<String>,

    // UI
    pub active_tab: Tab,
    pub settings_open: bool,
    pub context_menu: Option<ContextMenuState>,
    pub cursor_pos: Point,
    pub input_focused: bool,

    // Scan
    pub scan_cancel: ScanCancellation,
    pub scan_progress: Option<nullamp_core::models::ScanProgress>,
    pub is_scanning: bool,
    pub scan_modal_open: bool,
    pub scan_recent_files: std::collections::VecDeque<String>,

    // Voice
    pub mic: Option<MicCapture>,
    pub whisper: Option<Arc<Mutex<WhisperState>>>,
    pub voice_model_missing: bool,
    pub is_recording: bool,
    pub waveform_phase: f32,
    pub voice_transcript: String,
    pub voice_status: String,

    // Whisper model management
    pub model_infos: Vec<WhisperModelInfo>,
    pub model_download_progress: HashMap<String, (f32, u64, u64)>,
    pub active_model_id: Option<String>,

    // Settings UI
    pub show_gemini_key: bool,

    // i18n
    pub translations: std::collections::HashMap<
        &'static str,
        std::collections::HashMap<&'static str, &'static str>,
    >,
}

// ── Scan streaming helpers ────────────────────────────────────────────────────

/// Items flowing through the scan progress channel.
enum ScanStreamItem {
    Progress(nullamp_core::models::ScanProgress),
    Done(nullamp_core::models::SyncStatus),
}

/// Unfold state machine: Boot → spawn the blocking task, Drain → read channel events.
enum ScanUnfoldState {
    Boot {
        db: DbState,
        folder: String,
        cancel: nullamp_core::models::ScanCancellation,
    },
    Drain(futures::channel::mpsc::Receiver<ScanStreamItem>),
}

/// Bridges `ProgressEmitter` (called from rayon threads) into an async mpsc channel.
/// Mutex makes it `Sync` so rayon threads can call `emit` concurrently.
struct ChannelEmitter {
    sender: std::sync::Mutex<futures::channel::mpsc::Sender<ScanStreamItem>>,
}

impl ChannelEmitter {
    fn new(sender: futures::channel::mpsc::Sender<ScanStreamItem>) -> Self {
        Self {
            sender: std::sync::Mutex::new(sender),
        }
    }
}

impl nullamp_core::indexer::ProgressEmitter for ChannelEmitter {
    fn emit(&self, progress: &nullamp_core::models::ScanProgress) {
        if let Ok(mut tx) = self.sender.lock() {
            let _ = tx.try_send(ScanStreamItem::Progress(progress.clone()));
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────

impl Nullamp {
    pub fn new() -> (Self, Task<Message>) {
        // Database path
        let db_path = Settings::default_path()
            .parent()
            .unwrap_or(std::path::Path::new("."))
            .join("library.db");

        let conn = nullamp_core::db::init_db(&db_path).expect("Failed to initialize database");
        let db: DbState = Arc::new(Mutex::new(conn));

        // Settings
        let settings_path = Settings::default_path();
        let settings = Settings::load(&settings_path).unwrap_or_default();

        // Audio player
        let player = AudioPlayer::new().expect("Failed to initialize audio player");

        // Microphone
        let mic = MicCapture::new().ok();

        // Whisper model discovery
        let models_dir = Settings::default_path()
            .parent()
            .unwrap_or(std::path::Path::new("."))
            .join("models");
        let whisper = find_whisper_model(&models_dir);
        let voice_model_missing = whisper.is_none();

        // Detect which model was auto-loaded (first match in priority order)
        let active_model_id: Option<String> = if whisper.is_some() {
            WHISPER_MODELS
                .iter()
                .find(|(_, filename, _)| models_dir.join(filename).exists())
                .map(|(id, _, _)| id.to_string())
        } else {
            None
        };
        let model_infos = scan_model_infos(&models_dir, active_model_id.as_deref());

        let app = Self {
            db: db.clone(),
            settings,
            settings_path,
            player,
            playlist: Vec::new(),
            current_index: None,
            shuffle: false,
            repeat: RepeatMode::Off,
            all_tracks: Vec::new(),
            artists: Vec::new(),
            search_query: String::new(),
            search_results: Vec::new(),
            library_view_mode: LibraryViewMode::AllTracks,
            expanded_artists: HashSet::new(),
            active_tab: Tab::Library,
            settings_open: false,
            context_menu: None,
            cursor_pos: Point::ORIGIN,
            input_focused: false,
            scan_cancel: ScanCancellation::new(),
            scan_progress: None,
            is_scanning: false,
            scan_modal_open: false,
            scan_recent_files: std::collections::VecDeque::new(),
            mic,
            whisper,
            voice_model_missing,
            is_recording: false,
            waveform_phase: 0.0,
            voice_transcript: String::new(),
            voice_status: String::new(),
            model_infos,
            model_download_progress: HashMap::new(),
            active_model_id,
            show_gemini_key: false,
            translations: nullamp_core::i18n::translations(),
        };

        // Load library on startup
        let startup_task = Task::perform(
            {
                let db = db.clone();
                async move {
                    let conn = db.lock().map_err(|e| e.to_string()).ok();
                    conn.and_then(|c| db::get_tracks(&c, None, None).ok())
                        .unwrap_or_default()
                }
            },
            Message::LibraryLoaded,
        );

        (app, startup_task)
    }

    pub fn title(&self) -> String {
        if let Some(idx) = self.current_index {
            if let Some(track) = self.playlist.get(idx) {
                let title = track.title.as_deref().unwrap_or("Unknown");
                let artist = track.artist.as_deref().unwrap_or("Unknown");
                return format!("{} - {} | Nullamp", title, artist);
            }
        }
        "Nullamp".to_string()
    }

    pub fn theme(&self) -> Theme {
        theme::nullamp_theme()
    }

    /// Look up a translated string for the current language, falling back to English.
    pub fn t(&self, key: &'static str) -> &str {
        nullamp_core::i18n::t(&self.translations, &self.settings.language, key)
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let tick = if self.player.state() == PlayState::Playing || self.is_recording {
            iced::time::every(Duration::from_millis(100)).map(|_| Message::Tick(()))
        } else {
            Subscription::none()
        };

        let keyboard = iced::keyboard::on_key_press(keyboard_handler);

        // Track cursor position for context menu placement
        let cursor_track = iced::event::listen_with(|event, _status, _id| {
            if let iced::Event::Mouse(iced::mouse::Event::CursorMoved { position }) = event {
                Some(Message::CursorMoved(position))
            } else {
                None
            }
        });

        Subscription::batch([tick, keyboard, cursor_track])
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            // ── Playback ──
            Message::Play => {
                self.player.play();
            }
            Message::Pause => {
                self.player.pause();
            }
            Message::TogglePlayPause => {
                if self.input_focused || self.settings_open {
                    return Task::none();
                }
                if self.player.state() == PlayState::Stopped && self.current_index.is_none() {
                    if !self.playlist.is_empty() {
                        self.current_index = Some(0);
                        self.play_current();
                    }
                } else {
                    self.player.toggle_play_pause();
                }
            }
            Message::Stop => {
                self.player.stop();
                self.current_index = None;
            }
            Message::NextTrack => {
                if !self.input_focused && !self.settings_open {
                    self.advance_track(1);
                }
            }
            Message::PrevTrack => {
                if !self.input_focused && !self.settings_open {
                    self.advance_track(-1);
                }
            }
            Message::Seek(pct) => {
                if let Some(idx) = self.current_index {
                    if let Some(track) = self.playlist.get(idx) {
                        if let Some(dur) = track.duration_secs {
                            let pos = Duration::from_secs_f64(dur * pct as f64);
                            let _ = self.player.seek(pos);
                        }
                    }
                }
            }
            Message::SeekDelta(delta_secs) => {
                if self.input_focused || self.settings_open {
                    return Task::none();
                }
                let current = self.player.position().as_secs_f64();
                let max = self
                    .current_index
                    .and_then(|i| self.playlist.get(i))
                    .and_then(|t| t.duration_secs)
                    .unwrap_or(0.0);
                let new_pos = (current + delta_secs as f64).max(0.0).min(max);
                let _ = self.player.seek(Duration::from_secs_f64(new_pos));
            }
            Message::VolumeChanged(vol) => {
                self.player.set_volume(vol / 100.0);
                self.settings.volume = vol / 100.0;
            }
            Message::ToggleShuffle => {
                self.shuffle = !self.shuffle;
            }
            Message::CycleRepeat => {
                self.repeat = self.repeat.cycle();
            }
            Message::TrackEnded => {
                self.advance_track(1);
            }
            Message::Tick(_) => {
                if self.player.is_track_finished() {
                    self.player.mark_stopped();
                    return Task::done(Message::TrackEnded);
                }
                // Advance waveform animation
                if self.is_recording {
                    self.waveform_phase += 0.3;
                }
            }

            // ── Library ──
            Message::LibraryLoaded(tracks) => {
                self.all_tracks = tracks;
                let db = self.db.clone();
                return Task::perform(
                    async move {
                        let conn = db.lock().map_err(|e| e.to_string()).ok();
                        conn.and_then(|c| db::get_all_artists(&c).ok())
                            .unwrap_or_default()
                    },
                    Message::ArtistsLoaded,
                );
            }
            Message::ArtistsLoaded(artists) => {
                self.artists = artists;
            }
            Message::SearchQueryChanged(query) => {
                self.search_query = query.clone();
                if query.is_empty() {
                    self.search_results.clear();
                    return Task::none();
                }
                let db = self.db.clone();
                return Task::perform(
                    async move {
                        let q = SearchQuery {
                            text: Some(query),
                            artist: None,
                            album: None,
                            genre: None,
                        };
                        let conn = db.lock().map_err(|e| e.to_string()).ok();
                        conn.and_then(|c| db::search_tracks(&c, &q).ok())
                            .unwrap_or_default()
                    },
                    Message::SearchResults,
                );
            }
            Message::SearchResults(results) => {
                self.search_results = results;
            }
            Message::TrackAction(source, index, action) => {
                self.context_menu = None;
                let track = match source {
                    TrackSource::Library => self.all_tracks.get(index).cloned(),
                    TrackSource::Search => self.search_results.get(index).cloned(),
                    TrackSource::Queue => self.playlist.get(index).cloned(),
                };
                if let Some(track) = track {
                    match action {
                        TrackAction::PlayNow => match source {
                            TrackSource::Queue => {
                                // Already in playlist — just jump to position.
                                self.current_index = Some(index);
                                self.play_current();
                            }
                            TrackSource::Library => {
                                // Replace playlist with full library so Prev/Next work.
                                self.playlist = self.all_tracks.clone();
                                self.current_index = Some(index);
                                self.play_current();
                            }
                            TrackSource::Search => {
                                // Replace playlist with current search results.
                                self.playlist = self.search_results.clone();
                                self.current_index = Some(index);
                                self.play_current();
                            }
                        },
                        TrackAction::PlayNext => {
                            let insert_at = self.current_index.map(|i| i + 1).unwrap_or(0);
                            self.playlist.insert(insert_at, track);
                        }
                        TrackAction::AddToQueue => {
                            self.playlist.push(track);
                        }
                        TrackAction::RemoveFromQueue => {
                            if index < self.playlist.len() {
                                self.playlist.remove(index);
                                if let Some(ci) = self.current_index {
                                    if index < ci {
                                        self.current_index = Some(ci - 1);
                                    } else if index == ci {
                                        self.current_index = None;
                                        self.player.stop();
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Message::SetLibraryViewMode(mode) => {
                self.library_view_mode = mode;
            }
            Message::ToggleArtistExpanded(artist) => {
                if self.expanded_artists.contains(&artist) {
                    self.expanded_artists.remove(&artist);
                } else {
                    self.expanded_artists.insert(artist);
                }
            }

            // ── Context menu ──
            Message::ContextMenuOpen { source, index } => {
                self.context_menu = Some(ContextMenuState {
                    source,
                    track_index: index,
                    position: self.cursor_pos,
                });
            }
            Message::ContextMenuClose => {
                self.context_menu = None;
            }

            // ── Queue operations ──
            Message::QueueAll => {
                self.playlist.extend(self.all_tracks.clone());
            }
            Message::QueueAllSearchResults => {
                self.playlist.extend(self.search_results.clone());
            }
            Message::JumpToQueueTrack(index) => {
                self.context_menu = None;
                if index < self.playlist.len() {
                    self.current_index = Some(index);
                    self.play_current();
                }
            }
            Message::ClearQueue => {
                self.playlist.clear();
                self.current_index = None;
                self.player.stop();
            }

            // ── Tabs ──
            Message::TabSelected(tab) => {
                self.active_tab = tab;
                self.context_menu = None;
            }

            // ── EQ ──
            Message::EqBandChanged(band, gain) => {
                self.player.eq_params().set_band(band, gain);
            }
            Message::EqPreampChanged(gain) => {
                self.player.eq_params().set_preamp(gain);
            }
            Message::EqToggled(enabled) => {
                self.player.eq_params().set_enabled(enabled);
            }
            Message::EqPresetSelected(name) => {
                self.player.eq_params().load_preset(&name);
            }

            // ── Settings ──
            Message::OpenSettings => {
                self.settings_open = true;
                self.context_menu = None;
            }
            Message::CloseSettings => {
                if self.scan_modal_open {
                    self.scan_modal_open = false;
                } else {
                    self.settings_open = false;
                    let _ = self.settings.save(&self.settings_path);
                }
            }
            Message::PickMusicFolder => {
                return Task::perform(
                    async {
                        let folder = rfd::AsyncFileDialog::new()
                            .set_title("Select Music Folder")
                            .pick_folder()
                            .await;
                        folder.map(|f| f.path().to_string_lossy().to_string())
                    },
                    Message::MusicFolderPicked,
                );
            }
            Message::MusicFolderPicked(Some(path)) => {
                self.settings.music_folder = Some(path);
                let _ = self.settings.save(&self.settings_path);
                return Task::done(Message::ScanStart);
            }
            Message::MusicFolderPicked(None) => {}
            Message::GeminiApiKeyChanged(key) => {
                self.settings.gemini_api_key = key;
                let _ = self.settings.save(&self.settings_path);
            }
            Message::GeminiModelChanged(model) => {
                self.settings.gemini_model = model;
                let _ = self.settings.save(&self.settings_path);
            }
            Message::ShowGeminiKey(show) => {
                self.show_gemini_key = show;
            }
            Message::LanguageToggled => {
                self.settings.language = if self.settings.language == "en" {
                    "es".to_string()
                } else {
                    "en".to_string()
                };
                let _ = self.settings.save(&self.settings_path);
            }

            // ── Scan ──
            Message::ScanStart => {
                if let Some(ref folder) = self.settings.music_folder {
                    self.is_scanning = true;
                    self.scan_recent_files.clear();
                    self.scan_cancel.reset();
                    let db = self.db.clone();
                    let folder = folder.clone();
                    let cancel = self.scan_cancel.clone();

                    return Task::run(
                        futures::stream::unfold(
                            ScanUnfoldState::Boot { db, folder, cancel },
                            |state| async move {
                                match state {
                                    ScanUnfoldState::Boot { db, folder, cancel } => {
                                        let (mut completion_tx, rx) =
                                            futures::channel::mpsc::channel::<ScanStreamItem>(512);
                                        let progress_tx = completion_tx.clone();

                                        tokio::task::spawn(async move {
                                            let result = tokio::task::spawn_blocking(move || {
                                                let emitter = ChannelEmitter::new(progress_tx);
                                                let path = std::path::Path::new(&folder);
                                                nullamp_core::indexer::sync_library_parallel(
                                                    &db, path, &emitter, &cancel,
                                                )
                                            })
                                            .await;

                                            let status = result
                                                .ok()
                                                .and_then(|r| r.ok())
                                                .unwrap_or(nullamp_core::models::SyncStatus {
                                                    total_tracks: 0,
                                                    new_tracks: 0,
                                                    removed_tracks: 0,
                                                    scan_duration_ms: 0,
                                                });
                                            let _ = completion_tx
                                                .try_send(ScanStreamItem::Done(status));
                                        });

                                        Some((None, ScanUnfoldState::Drain(rx)))
                                    }
                                    ScanUnfoldState::Drain(mut rx) => match rx.next().await {
                                        Some(item) => {
                                            Some((Some(item), ScanUnfoldState::Drain(rx)))
                                        }
                                        None => None,
                                    },
                                }
                            },
                        )
                        .filter_map(|x| async move { x })
                        .map(|item| match item {
                            ScanStreamItem::Progress(p) => Message::ScanProgressUpdate(p),
                            ScanStreamItem::Done(s) => Message::ScanComplete(s),
                        }),
                        |msg| msg,
                    );
                }
            }
            Message::ScanProgressUpdate(progress) => {
                if !progress.current_file.is_empty() {
                    if self.scan_recent_files.len() >= 50 {
                        self.scan_recent_files.pop_front();
                    }
                    self.scan_recent_files
                        .push_back(progress.current_file.clone());
                }
                self.scan_progress = Some(progress);
            }
            Message::ScanComplete(status) => {
                self.is_scanning = false;
                log::info!(
                    "Scan complete: {} total, {} new, {} removed",
                    status.total_tracks,
                    status.new_tracks,
                    status.removed_tracks
                );
                let db = self.db.clone();
                return Task::perform(
                    async move {
                        let conn = db.lock().map_err(|e| e.to_string()).ok();
                        conn.and_then(|c| db::get_tracks(&c, None, None).ok())
                            .unwrap_or_default()
                    },
                    Message::LibraryLoaded,
                );
            }
            Message::ScanCancel => {
                self.scan_cancel.cancel();
            }
            Message::OpenScanModal => {
                self.scan_modal_open = true;
            }
            Message::CloseScanModal => {
                self.scan_modal_open = false;
            }

            // ── Whisper model management ──
            Message::WhisperModelDownload(id) => {
                if self.model_download_progress.contains_key(&id) {
                    return Task::none(); // already downloading
                }
                let Some(&(_, filename, _)) = WHISPER_MODELS
                    .iter()
                    .find(|(mid, _, _)| *mid == id.as_str())
                else {
                    return Task::none();
                };
                let url = format!(
                    "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/{}",
                    filename
                );
                let models_dir = Settings::default_path()
                    .parent()
                    .unwrap_or(std::path::Path::new("."))
                    .join("models");
                let dest = models_dir.join(filename);
                let id_clone = id.clone();
                self.model_download_progress.insert(id.clone(), (0.0, 0, 0));

                return Task::run(
                    futures::stream::unfold(
                        (url, dest, id_clone, false),
                        |(url, dest, id, done)| async move {
                            if done {
                                return None;
                            }
                            // Create models dir if needed
                            if let Some(parent) = dest.parent() {
                                let _ = tokio::fs::create_dir_all(parent).await;
                            }
                            // Stream download
                            let result = async {
                                use futures::StreamExt;
                                use tokio::io::AsyncWriteExt;
                                let resp = reqwest::get(&url).await?;
                                let total = resp.content_length().unwrap_or(0);
                                let mut file = tokio::fs::File::create(&dest).await?;
                                let mut stream = resp.bytes_stream();
                                let mut downloaded: u64 = 0;
                                let mut msgs: Vec<Message> = Vec::new();
                                while let Some(chunk) = stream.next().await {
                                    let bytes = chunk?;
                                    file.write_all(&bytes).await?;
                                    downloaded += bytes.len() as u64;
                                    let pct = if total > 0 {
                                        downloaded as f32 / total as f32
                                    } else {
                                        0.0
                                    };
                                    msgs.push(Message::WhisperModelDownloadProgress {
                                        model: id.clone(),
                                        percent: pct,
                                        bytes_downloaded: downloaded,
                                        total_bytes: total,
                                    });
                                }
                                file.flush().await?;
                                msgs.push(Message::WhisperModelDownloadComplete(id.clone()));
                                Ok::<_, Box<dyn std::error::Error + Send + Sync>>(msgs)
                            }
                            .await;
                            match result {
                                Ok(msgs) => {
                                    // Yield all messages as a vec wrapped in a single stream item
                                    Some((msgs, (url, dest, id, true)))
                                }
                                Err(e) => Some((
                                    vec![Message::WhisperModelDownloadError {
                                        model: id.clone(),
                                        error: e.to_string(),
                                    }],
                                    (url, dest, id, true),
                                )),
                            }
                        },
                    )
                    .flat_map(|msgs| futures::stream::iter(msgs)),
                    |msg| msg,
                );
            }
            Message::WhisperModelDownloadProgress {
                model,
                percent,
                bytes_downloaded,
                total_bytes,
            } => {
                self.model_download_progress
                    .insert(model, (percent, bytes_downloaded, total_bytes));
            }
            Message::WhisperModelDownloadComplete(id) => {
                self.model_download_progress.remove(&id);
                let models_dir = Settings::default_path()
                    .parent()
                    .unwrap_or(std::path::Path::new("."))
                    .join("models");
                self.model_infos = scan_model_infos(&models_dir, Some(&id));
                // Auto-load if no whisper model yet
                if self.whisper.is_none() {
                    self.whisper = find_whisper_model(&models_dir);
                    self.voice_model_missing = self.whisper.is_none();
                    self.active_model_id = Some(id.clone());
                    self.model_infos = scan_model_infos(&models_dir, Some(&id));
                }
            }
            Message::WhisperModelDownloadError { model, error } => {
                self.model_download_progress.remove(&model);
                self.voice_status = format!("Download failed: {error}");
            }
            Message::WhisperModelSwitch(id) => {
                let models_dir = Settings::default_path()
                    .parent()
                    .unwrap_or(std::path::Path::new("."))
                    .join("models");
                let Some(&(_, filename, _)) = WHISPER_MODELS
                    .iter()
                    .find(|(mid, _, _)| *mid == id.as_str())
                else {
                    return Task::none();
                };
                let path = models_dir.join(filename);
                if path.exists() {
                    self.whisper = Some(Arc::new(Mutex::new(WhisperState::new(path))));
                    self.voice_model_missing = false;
                    self.active_model_id = Some(id.clone());
                    self.model_infos = scan_model_infos(&models_dir, Some(&id));
                }
            }

            // ── Voice ──
            Message::VoiceStartRecording => {
                if self.voice_model_missing {
                    self.voice_status = "Voice model not found. See settings.".to_string();
                    return Task::none();
                }
                match self.mic.as_mut().map(|m| m.start()) {
                    Some(Ok(())) => {
                        self.is_recording = true;
                        self.waveform_phase = 0.0;
                        self.voice_status = "Listening...".to_string();
                        self.voice_transcript = String::new();
                    }
                    Some(Err(e)) => {
                        self.voice_status = format!("Mic error: {e}");
                    }
                    None => {
                        self.voice_status = "No microphone available".to_string();
                    }
                }
            }
            Message::VoiceStopRecording => {
                self.is_recording = false;
                let samples = self.mic.as_mut().map(|m| m.stop()).unwrap_or_default();
                if samples.is_empty() {
                    self.voice_status = "No audio captured".to_string();
                } else {
                    self.voice_status = "Transcribing...".to_string();
                    return Task::done(Message::VoiceRecordingFinished(samples));
                }
            }
            Message::VoiceRecordingFinished(samples) => {
                if let Some(ref whisper_arc) = self.whisper {
                    let whisper = Arc::clone(whisper_arc);
                    let lang = self.settings.language.clone();
                    return Task::perform(
                        async move {
                            tokio::task::spawn_blocking(move || {
                                let mut w = whisper.lock().unwrap();
                                if !w.is_loaded() {
                                    w.load_model()?;
                                }
                                w.transcribe(&samples, &lang)
                            })
                            .await
                            .unwrap_or_else(|e| Err(e.to_string()))
                        },
                        |result| match result {
                            Ok(text) => Message::VoiceTranscriptionComplete(text),
                            Err(e) => Message::VoiceError(e),
                        },
                    );
                } else {
                    self.voice_status = "Whisper model not loaded".to_string();
                }
            }
            Message::VoiceTranscriptionComplete(text) => {
                self.voice_transcript = text.clone();
                self.voice_status = "Processing intent...".to_string();

                // Try local intent first
                if let Some(intent) = nullamp_core::intent::parse_local_intent(&text) {
                    return Task::done(Message::VoiceIntent(intent));
                }

                // Gemini fallback
                if !self.settings.gemini_api_key.is_empty() {
                    let api_key = self.settings.gemini_api_key.clone();
                    let model = self.settings.gemini_model.clone();
                    let text_c = text.clone();
                    return Task::perform(
                        async move {
                            nullamp_core::intent::extract_intent(&api_key, &text_c, &model)
                                .await
                                .map_err(|e| e.to_string())
                        },
                        |result| match result {
                            Ok(intent) => Message::VoiceIntent(intent),
                            Err(e) => Message::VoiceError(e),
                        },
                    );
                }

                self.voice_status = "No intent recognized".to_string();
            }
            // Legacy alias kept for compatibility
            Message::VoiceTranscription(text) => {
                self.voice_transcript = text;
            }
            Message::VoiceIntent(intent) => {
                self.voice_status = nullamp_core::intent::describe_intent(&intent);
                match intent.action {
                    IntentAction::Next => return Task::done(Message::NextTrack),
                    IntentAction::Previous => return Task::done(Message::PrevTrack),
                    IntentAction::Pause => return Task::done(Message::Pause),
                    IntentAction::Resume => return Task::done(Message::Play),
                    IntentAction::Stop => return Task::done(Message::Stop),
                    IntentAction::VolumeUp => {
                        let v = (self.settings.volume * 100.0 + 10.0).min(100.0);
                        return Task::done(Message::VolumeChanged(v));
                    }
                    IntentAction::VolumeDown => {
                        let v = (self.settings.volume * 100.0 - 10.0).max(0.0);
                        return Task::done(Message::VolumeChanged(v));
                    }
                    IntentAction::Shuffle => return Task::done(Message::ToggleShuffle),
                    IntentAction::ClearQueue => return Task::done(Message::ClearQueue),
                    IntentAction::Search => {
                        if let Some(ref q) = intent.query {
                            let term = q
                                .free_text
                                .clone()
                                .or_else(|| q.artist.clone())
                                .or_else(|| q.title.clone())
                                .unwrap_or_default();
                            if !term.is_empty() {
                                // Populate search in the background — do NOT switch tabs.
                                // The user stays in the Voice tab; they can switch to Search
                                // manually if they want to browse results.
                                return Task::done(Message::SearchQueryChanged(term));
                            }
                        }
                    }
                    IntentAction::Play | IntentAction::Queue => {
                        let tracks = self.search_tracks_for_intent(intent.query.as_ref());
                        if !tracks.is_empty() {
                            if intent.action == IntentAction::Play {
                                self.playlist = tracks;
                                self.current_index = Some(0);
                                self.play_current();
                            } else {
                                self.playlist.extend(tracks);
                            }
                        } else {
                            self.voice_status = "No tracks found".to_string();
                        }
                    }
                }
            }
            Message::VoiceError(err) => {
                log::error!("[Voice] {err}");
                self.voice_status = format!("Error: {err}");
                self.is_recording = false;
            }
            Message::VoiceTextCommand(cmd) => {
                self.voice_transcript = cmd.clone();
                if let Some(intent) = nullamp_core::intent::parse_local_intent(&cmd) {
                    return Task::done(Message::VoiceIntent(intent));
                }
                if !self.settings.gemini_api_key.is_empty() {
                    self.voice_status = "Processing intent\u{2026}".to_string();
                    let api_key = self.settings.gemini_api_key.clone();
                    let model = self.settings.gemini_model.clone();
                    return Task::perform(
                        async move {
                            nullamp_core::intent::extract_intent(&api_key, &cmd, &model)
                                .await
                                .map_err(|e| e.to_string())
                        },
                        |result| match result {
                            Ok(intent) => Message::VoiceIntent(intent),
                            Err(e) => Message::VoiceError(e),
                        },
                    );
                }
            }

            // ── System ──
            Message::CursorMoved(pos) => {
                self.cursor_pos = pos;
            }
            Message::DragWindow => {
                return iced::window::get_oldest().and_then(|id| iced::window::drag(id));
            }
            Message::CloseWindow => {
                return iced::window::get_oldest().and_then(|id| iced::window::close(id));
            }
            Message::MinimizeWindow => {
                return iced::window::get_oldest().and_then(|id| iced::window::minimize(id, true));
            }
            Message::Noop => {}
        }
        Task::none()
    }

    pub fn view(&self) -> Element<'_, Message> {
        let title_bar = views::title_bar(self);
        let player_bar = views::player_bar(self);
        let tab_bar = views::tab_bar(self);
        let status_bar = views::status_bar(self);
        let queue_panel = views::queue_panel(self);

        // Main UI always renders at full natural height — no conditional swap.
        let main = column![
            title_bar,
            player_bar,
            tab_bar,
            container(views::tab_content(self))
                .width(Length::Fill)
                .height(Length::Fill),
            queue_panel,
            status_bar,
        ]
        .width(Length::Fill)
        .height(Length::Fill);

        let base = container(main)
            .style(|_theme| container::Style {
                background: Some(theme::BG_PRIMARY.into()),
                ..Default::default()
            })
            .width(Length::Fill)
            .height(Length::Fill);

        // Build optional overlay layers
        let settings_overlay: Option<Element<'_, Message>> = if self.settings_open {
            // Semi-transparent backdrop that closes the modal on click
            // Backdrop absorbs ALL mouse interactions — only the Close button closes the modal.
            let backdrop = iced::widget::mouse_area(
                container("")
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .style(|_theme| container::Style {
                        background: Some(
                            iced::Color {
                                r: 0.0,
                                g: 0.0,
                                b: 0.0,
                                a: 0.65,
                            }
                            .into(),
                        ),
                        ..Default::default()
                    }),
            )
            .on_press(Message::Noop)
            .on_release(Message::Noop)
            .on_right_press(Message::Noop)
            .on_middle_press(Message::Noop)
            .on_move(|_| Message::Noop);

            // Centered settings panel (fixed width, shrink height)
            let panel = container(views::settings_view(self))
                .width(Length::Fixed(460.0))
                .style(|_theme| container::Style {
                    background: Some(theme::BG_SURFACE.into()),
                    border: iced::Border {
                        color: theme::BORDER_FRAME,
                        width: 1.0,
                        radius: 6.into(),
                    },
                    ..Default::default()
                });

            let centered = container(panel)
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill);

            Some(stack![backdrop, centered].into())
        } else {
            None
        };

        let ctx_overlay: Option<(Element<'_, Message>, Element<'_, Message>)> =
            self.context_menu.as_ref().map(|ctx| {
                let menu = views::context_menu::context_menu_widget(ctx, self);
                let cx = ctx.position.x;
                let cy = ctx.position.y;
                let positioned = container(iced::widget::column![
                    iced::widget::Space::with_height(Length::Fixed(cy)),
                    iced::widget::row![iced::widget::Space::with_width(Length::Fixed(cx)), menu,],
                ])
                .width(Length::Fill)
                .height(Length::Fill);

                let dismiss = iced::widget::mouse_area(
                    container("").width(Length::Fill).height(Length::Fill),
                )
                .on_press(Message::ContextMenuClose);

                (dismiss.into(), positioned.into())
            });

        // Scan modal — same backdrop + panel pattern as settings, mutually exclusive
        let scan_modal_overlay: Option<Element<'_, Message>> =
            if self.scan_modal_open && !self.settings_open {
                let backdrop = iced::widget::mouse_area(
                    container("")
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .style(|_theme| container::Style {
                            background: Some(
                                iced::Color {
                                    r: 0.0,
                                    g: 0.0,
                                    b: 0.0,
                                    a: 0.65,
                                }
                                .into(),
                            ),
                            ..Default::default()
                        }),
                )
                .on_press(Message::CloseScanModal)
                .on_release(Message::Noop)
                .on_right_press(Message::Noop)
                .on_middle_press(Message::Noop)
                .on_move(|_| Message::Noop);

                let panel = container(views::scan_modal_view(self))
                    .width(Length::Fixed(480.0))
                    .style(|_theme| container::Style {
                        background: Some(theme::BG_SURFACE.into()),
                        border: iced::Border {
                            color: theme::BORDER_FRAME,
                            width: 1.0,
                            radius: 6.into(),
                        },
                        ..Default::default()
                    });

                let centered = container(panel)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .center_x(Length::Fill)
                    .center_y(Length::Fill);

                Some(stack![backdrop, centered].into())
            } else {
                None
            };

        let modal_overlay = settings_overlay.or(scan_modal_overlay);

        match (modal_overlay, ctx_overlay) {
            (Some(m), Some((d, p))) => stack![base, m, d, p].into(),
            (Some(m), None) => stack![base, m].into(),
            (None, Some((d, p))) => stack![base, d, p].into(),
            (None, None) => base.into(),
        }
    }

    // ── Internal helpers ──

    fn play_current(&mut self) {
        if let Some(idx) = self.current_index {
            if let Some(track) = self.playlist.get(idx) {
                let path = std::path::Path::new(&track.file_path);
                if let Err(e) = self.player.load_and_play(path) {
                    log::error!("Failed to play track: {e}");
                }
            }
        }
    }

    fn advance_track(&mut self, direction: i32) {
        if self.playlist.is_empty() {
            return;
        }

        if self.repeat == RepeatMode::One {
            self.play_current();
            return;
        }

        let len = self.playlist.len() as i32;
        let current = self.current_index.unwrap_or(0) as i32;

        let next = if self.shuffle {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut h = DefaultHasher::new();
            std::time::Instant::now().hash(&mut h);
            (h.finish() as i32 % len).unsigned_abs() as i32
        } else {
            let n = current + direction;
            if n < 0 {
                if self.repeat == RepeatMode::All {
                    len - 1
                } else {
                    return;
                }
            } else if n >= len {
                if self.repeat == RepeatMode::All {
                    0
                } else {
                    self.player.stop();
                    return;
                }
            } else {
                n
            }
        };

        self.current_index = Some(next as usize);
        self.play_current();
    }

    /// Synchronous track search used by voice intent execution.
    fn search_tracks_for_intent(
        &self,
        query: Option<&nullamp_core::models::IntentQuery>,
    ) -> Vec<Track> {
        let Some(q) = query else {
            return self.all_tracks.clone();
        };
        let text = q
            .free_text
            .clone()
            .or_else(|| q.artist.clone())
            .or_else(|| q.title.clone())
            .or_else(|| q.album.clone())
            .or_else(|| q.genre.clone());
        let Some(term) = text else {
            return self.all_tracks.clone();
        };
        // Use only full-text OR search across all columns — structured fields (artist/album/genre)
        // create AND conditions that fail when tracks lack complete metadata tags.
        let search_q = SearchQuery {
            text: Some(term),
            artist: None,
            album: None,
            genre: None,
        };
        self.db
            .lock()
            .ok()
            .and_then(|c| db::search_tracks(&c, &search_q).ok())
            .unwrap_or_default()
    }
}

/// Scan which Whisper models are downloaded and mark the active one.
fn scan_model_infos(
    models_dir: &std::path::Path,
    active_id: Option<&str>,
) -> Vec<WhisperModelInfo> {
    WHISPER_MODELS
        .iter()
        .map(|(id, filename, size_mb)| {
            let downloaded = models_dir.join(filename).exists();
            WhisperModelInfo {
                id,
                size_mb: *size_mb,
                downloaded,
                is_active: active_id == Some(id),
            }
        })
        .collect()
}

/// Find the first available Whisper model in the models directory.
fn find_whisper_model(models_dir: &std::path::Path) -> Option<Arc<Mutex<WhisperState>>> {
    for name in [
        "ggml-small.bin",
        "ggml-base.bin",
        "ggml-tiny.bin",
        "ggml-medium.bin",
    ] {
        let path = models_dir.join(name);
        if path.exists() {
            log::info!("Found Whisper model: {}", path.display());
            return Some(Arc::new(Mutex::new(WhisperState::new(path))));
        }
    }
    None
}
