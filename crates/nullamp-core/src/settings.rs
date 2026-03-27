use crate::error::AppError;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Application settings persisted as a JSON file.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub gemini_api_key: String,
    pub gemini_model: String,
    pub music_folder: Option<String>,
    pub language: String,
    pub volume: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            gemini_api_key: String::new(),
            gemini_model: "gemini-3.1-flash-lite-preview".to_string(),
            music_folder: None,
            language: "en".to_string(),
            volume: 0.8,
        }
    }
}

impl Settings {
    /// Default path for the settings file in the app data directory.
    pub fn default_path() -> PathBuf {
        let data_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("nullamp");
        data_dir.join("settings.json")
    }

    /// Load settings from a JSON file, returning defaults if the file doesn't exist.
    /// Migrates any stored model ID that is no longer in the supported list to the default.
    pub fn load(path: &Path) -> Result<Self, AppError> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let contents = std::fs::read_to_string(path)?;
        let mut settings: Self = serde_json::from_str(&contents)
            .map_err(|e| AppError::Other(format!("Invalid settings JSON: {e}")))?;

        const VALID_MODELS: &[&str] = &[
            "gemini-3.1-flash-lite-preview",
            "gemini-3-flash-preview",
            "gemini-2.5-flash",
            "gemini-2.5-flash-lite",
        ];
        if !VALID_MODELS.contains(&settings.gemini_model.as_str()) {
            settings.gemini_model = Self::default().gemini_model;
        }

        Ok(settings)
    }

    /// Save settings to a JSON file, creating parent directories if needed.
    pub fn save(&self, path: &Path) -> Result<(), AppError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| AppError::Other(format!("Failed to serialize settings: {e}")))?;
        std::fs::write(path, json)?;
        Ok(())
    }
}
