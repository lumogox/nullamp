use crate::eq::{EqParams, EqSource};
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

/// Playback state observable by the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayState {
    Stopped,
    Playing,
    Paused,
}

/// Native audio player backed by rodio.
/// Reads files directly from disk (no IPC or Blob URLs).
pub struct AudioPlayer {
    _stream: OutputStream,
    stream_handle: OutputStreamHandle,
    sink: Sink,
    eq_params: Arc<EqParams>,
    volume: f32, // 0.0 – 1.0
    current_path: Option<PathBuf>,
    state: PlayState,
}

impl AudioPlayer {
    /// Create a new audio player with default output device.
    pub fn new() -> Result<Self, String> {
        let (stream, handle) =
            OutputStream::try_default().map_err(|e| format!("Audio output init failed: {e}"))?;
        let sink = Sink::try_new(&handle).map_err(|e| format!("Sink creation failed: {e}"))?;
        sink.pause(); // Start paused

        Ok(Self {
            _stream: stream,
            stream_handle: handle,
            sink,
            eq_params: Arc::new(EqParams::new()),
            volume: 0.8,
            current_path: None,
            state: PlayState::Stopped,
        })
    }

    /// Get a reference to the shared EQ parameters (for UI control).
    pub fn eq_params(&self) -> &Arc<EqParams> {
        &self.eq_params
    }

    /// Load and start playing a track from disk.
    pub fn load_and_play(&mut self, path: &Path) -> Result<(), String> {
        // Stop current playback and create a fresh sink
        self.sink.stop();
        self.sink =
            Sink::try_new(&self.stream_handle).map_err(|e| format!("Sink creation failed: {e}"))?;
        self.sink.set_volume(self.volume);

        // Open file and decode — wrap in catch_unwind because some rodio/symphonia
        // decoder paths call unreachable!() on malformed files instead of returning Err.
        let file =
            File::open(path).map_err(|e| format!("Failed to open {}: {e}", path.display()))?;
        let buf = BufReader::new(file);
        let source = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| Decoder::new(buf)))
            .map_err(|_| format!("Decoder panicked on: {}", path.display()))?
            .map_err(|e| format!("Failed to decode {}: {e}", path.display()))?;

        // Wrap with EQ processing and append to sink
        let eq_source = EqSource::new(source.convert_samples::<f32>(), Arc::clone(&self.eq_params));
        self.sink.append(eq_source);

        self.current_path = Some(path.to_path_buf());
        self.state = PlayState::Playing;

        Ok(())
    }

    pub fn play(&mut self) {
        if self.state == PlayState::Paused {
            self.sink.play();
            self.state = PlayState::Playing;
        }
    }

    pub fn pause(&mut self) {
        if self.state == PlayState::Playing {
            self.sink.pause();
            self.state = PlayState::Paused;
        }
    }

    pub fn stop(&mut self) {
        self.sink.stop();
        self.current_path = None;
        self.state = PlayState::Stopped;
    }

    pub fn toggle_play_pause(&mut self) {
        match self.state {
            PlayState::Playing => self.pause(),
            PlayState::Paused => self.play(),
            PlayState::Stopped => {}
        }
    }

    /// Seek to a position. Requires reloading the file from disk.
    pub fn seek(&mut self, position: Duration) -> Result<(), String> {
        if let Err(e) = self.sink.try_seek(position) {
            // Fallback: reload file and skip to position
            log::warn!("Sink seek failed ({e}), reloading file");
            if let Some(ref path) = self.current_path.clone() {
                let was_playing = self.state == PlayState::Playing;
                self.load_and_play(path)?;
                if let Err(e2) = self.sink.try_seek(position) {
                    log::warn!("Seek after reload also failed: {e2}");
                }
                if !was_playing {
                    self.pause();
                }
            }
        }
        Ok(())
    }

    /// Set volume (0.0 to 1.0).
    pub fn set_volume(&mut self, vol: f32) {
        self.volume = vol.clamp(0.0, 1.0);
        self.sink.set_volume(self.volume);
    }

    pub fn volume(&self) -> f32 {
        self.volume
    }

    /// Get current playback position.
    pub fn position(&self) -> Duration {
        self.sink.get_pos()
    }

    /// Check if the track has finished playing (sink is empty after it was playing).
    pub fn is_track_finished(&self) -> bool {
        self.state == PlayState::Playing && self.sink.empty()
    }

    pub fn state(&self) -> PlayState {
        self.state
    }

    /// Mark state as stopped (call after detecting track end).
    pub fn mark_stopped(&mut self) {
        self.state = PlayState::Stopped;
        self.current_path = None;
    }

    pub fn current_path(&self) -> Option<&Path> {
        self.current_path.as_deref()
    }
}
