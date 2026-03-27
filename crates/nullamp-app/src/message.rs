use iced::Point;
use nullamp_core::models::{MusicIntent, ScanProgress, SyncStatus, Track};

/// Metadata for a single Whisper model (one of 4 supported sizes).
#[derive(Debug, Clone)]
pub struct WhisperModelInfo {
    pub id: &'static str, // "tiny" | "base" | "small" | "medium"
    pub size_mb: u32,
    pub downloaded: bool,
    pub is_active: bool,
}

/// Active tab in the main UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Library,
    Search,
    Voice,
    Equalizer,
}

/// Repeat mode for the queue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepeatMode {
    Off,
    All,
    One,
}

impl RepeatMode {
    pub fn cycle(self) -> Self {
        match self {
            Self::Off => Self::All,
            Self::All => Self::One,
            Self::One => Self::Off,
        }
    }
}

/// Where a context menu track action originated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackSource {
    Library,
    Search,
    Queue,
}

/// Track context actions (right-click / inline button).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackAction {
    PlayNow,
    PlayNext,
    AddToQueue,
    RemoveFromQueue,
}

/// Library view mode toggle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LibraryViewMode {
    AllTracks,
    ArtistTree,
}

/// All application messages (Elm architecture).
#[derive(Debug, Clone)]
pub enum Message {
    // ── Playback ──
    Play,
    Pause,
    TogglePlayPause,
    Stop,
    NextTrack,
    PrevTrack,
    Seek(f32),
    SeekDelta(f32), // seek by ±seconds
    VolumeChanged(f32),
    ToggleShuffle,
    CycleRepeat,
    TrackEnded,
    Tick(()),

    // ── Library ──
    LibraryLoaded(Vec<Track>),
    ArtistsLoaded(Vec<String>),
    SearchQueryChanged(String),
    SearchResults(Vec<Track>),
    /// Play-now click on a track row (single click).
    TrackAction(TrackSource, usize, TrackAction),
    SetLibraryViewMode(LibraryViewMode),
    ToggleArtistExpanded(String),

    // ── Context menu ──
    ContextMenuOpen {
        source: TrackSource,
        index: usize,
    },
    ContextMenuClose,

    // ── Queue operations ──
    QueueAll,
    QueueAllSearchResults,
    JumpToQueueTrack(usize),
    ClearQueue,

    // ── Tabs ──
    TabSelected(Tab),

    // ── EQ ──
    EqBandChanged(usize, f32),
    EqPreampChanged(f32),
    EqToggled(bool),
    EqPresetSelected(String),

    // ── Whisper model management ──
    WhisperModelDownload(String),
    WhisperModelDownloadProgress {
        model: String,
        percent: f32,
        bytes_downloaded: u64,
        total_bytes: u64,
    },
    WhisperModelDownloadComplete(String),
    WhisperModelDownloadError {
        model: String,
        error: String,
    },
    WhisperModelSwitch(String),

    // ── Voice ──
    VoiceStartRecording,
    VoiceStopRecording,
    VoiceRecordingFinished(Vec<f32>),
    VoiceTranscriptionComplete(String),
    VoiceTranscription(String),
    VoiceIntent(MusicIntent),
    VoiceError(String),
    VoiceTextCommand(String),

    // ── Scan ──
    ScanStart,
    ScanComplete(SyncStatus),
    ScanProgressUpdate(ScanProgress),
    ScanCancel,
    OpenScanModal,
    CloseScanModal,

    // ── Settings ──
    OpenSettings,
    CloseSettings,
    PickMusicFolder,
    MusicFolderPicked(Option<String>),
    GeminiApiKeyChanged(String),
    GeminiModelChanged(String),
    ShowGeminiKey(bool),
    LanguageToggled,

    // ── System ──
    CursorMoved(Point),
    DragWindow,
    CloseWindow,
    MinimizeWindow,
    Noop,
}
