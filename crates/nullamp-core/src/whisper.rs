use std::path::PathBuf;
use std::sync::Mutex;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

pub struct WhisperState {
    context: Option<WhisperContext>,
    model_path: PathBuf,
}

impl WhisperState {
    pub fn new(model_path: PathBuf) -> Self {
        Self {
            context: None,
            model_path,
        }
    }

    pub fn model_path(&self) -> &PathBuf {
        &self.model_path
    }

    pub fn is_model_available(&self) -> bool {
        self.model_path.exists()
    }

    pub fn is_loaded(&self) -> bool {
        self.context.is_some()
    }

    pub fn change_model(&mut self, model_path: PathBuf) -> Result<(), String> {
        if self.context.is_some() && self.model_path == model_path {
            return Ok(()); // already loaded
        }
        self.context = None;
        self.model_path = model_path;
        if self.is_model_available() {
            self.load_model()
        } else {
            Err(format!(
                "Model file not found: {}",
                self.model_path.display()
            ))
        }
    }

    pub fn load_model(&mut self) -> Result<(), String> {
        if !self.is_model_available() {
            return Err(format!(
                "Model file not found: {}",
                self.model_path.display()
            ));
        }

        log::info!("Loading Whisper model from: {}", self.model_path.display());

        let ctx = WhisperContext::new_with_params(
            self.model_path.to_str().ok_or("Invalid model path")?,
            WhisperContextParameters::default(),
        )
        .map_err(|e| format!("Failed to load Whisper model: {e}"))?;

        self.context = Some(ctx);
        log::info!("Whisper model loaded successfully");
        Ok(())
    }

    pub fn transcribe(&self, audio_data: &[f32], language: &str) -> Result<String, String> {
        let ctx = self
            .context
            .as_ref()
            .ok_or("Whisper model not loaded. Download it first.")?;

        let mut state = ctx.create_state().map_err(|e| e.to_string())?;

        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_language(Some(language));
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);

        state
            .full(params, audio_data)
            .map_err(|e| format!("Transcription failed: {e}"))?;

        let num_segments = state.full_n_segments();
        let mut text = String::new();
        for i in 0..num_segments {
            if let Some(segment) = state.get_segment(i) {
                let segment_text = segment.to_str_lossy().map_err(|e| e.to_string())?;
                text.push_str(&segment_text);
            }
        }

        Ok(text.trim().to_string())
    }
}

pub type SharedWhisperState = Mutex<WhisperState>;

/// Available Whisper model catalog.
pub const MODEL_CATALOG: &[(&str, &str)] = &[
    (
        "tiny",
        "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin",
    ),
    (
        "base",
        "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin",
    ),
    (
        "small",
        "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin",
    ),
    (
        "medium",
        "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.bin",
    ),
];
