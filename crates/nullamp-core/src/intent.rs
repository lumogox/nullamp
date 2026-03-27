use crate::error::AppError;
use crate::models::{IntentAction, IntentQuery, MusicIntent};

const SYSTEM_PROMPT: &str = r#"You are a music player voice assistant. Parse the user's voice command into a structured JSON music intent.
The user may speak in English or Spanish. Always respond in English JSON regardless of input language.

The JSON must match this exact schema:
{
  "action": "play" | "queue" | "search" | "pause" | "resume" | "next" | "previous" | "stop" | "volume_up" | "volume_down" | "shuffle" | "clear_queue",
  "query": {
    "artist": "string or omit",
    "title": "string or omit",
    "album": "string or omit",
    "genre": "string or omit",
    "mood": "string or omit",
    "freeText": "string or omit"
  }
}

Action rules:
- "play" -> find ALL matching tracks, replace the queue with them, and start playing.
- "queue" -> find ALL matching tracks and ADD them to the existing queue without interrupting playback.
- "search" -> just show results without touching the queue.
- "pause" / "resume" / "next" / "previous" / "stop" -> transport controls, no query needed.
- "volume_up" / "volume_down" -> volume control, no query needed.
- "shuffle" -> toggle shuffle mode, no query needed.
- "clear_queue" -> empty the queue, no query needed.

Query field rules:
- Put the band/artist name in "artist"
- Put the song name in "title"
- Put the album name in "album"
- Put the music style in "genre" (rock, jazz, blues, pop, classical, etc.)
- Put mood/vibe descriptions in "mood" (chill, energetic, sad, happy, etc.)
- If you can't categorize it, use "freeText"
- When the user says "all music from X" or "everything by X" -> use artist: "X" with action "play"

Return ONLY valid JSON. No markdown, no explanation, no code fences."#;

/// Extract a music intent from transcribed text using the Gemini API.
pub async fn extract_intent(
    api_key: &str,
    text: &str,
    model: &str,
) -> Result<MusicIntent, AppError> {
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent?key={api_key}"
    );

    let body = serde_json::json!({
        "contents": [{
            "parts": [{
                "text": format!("{SYSTEM_PROMPT}\n\nVoice command: \"{text}\"")
            }]
        }],
        "generationConfig": {
            "responseMimeType": "application/json"
        }
    });

    let client = reqwest::Client::new();
    let response = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| AppError::Other(format!("Gemini request failed: {e}")))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(AppError::Other(format!(
            "Gemini API error {status}: {body}"
        )));
    }

    let json: serde_json::Value = response.json().await?;
    let raw = json["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .ok_or_else(|| AppError::Other("No text in Gemini response".into()))?;

    log::debug!("[Gemini] raw response: {raw}");

    // Extract the JSON object even if the model wrapped it in markdown fences or added prose.
    let text = extract_json_object(raw).ok_or_else(|| {
        log::error!("[Gemini] no JSON object in response: {raw}");
        AppError::Other(format!("No JSON object found in Gemini response: {raw}"))
    })?;

    log::debug!("[Gemini] extracted JSON: {text}");

    let intent: MusicIntent = serde_json::from_str(text).map_err(|e| {
        log::error!("[Gemini] parse error: {e} — JSON was: {text}");
        AppError::Other(format!("Failed to parse intent JSON: {e}"))
    })?;

    Ok(intent)
}

/// Extract the first balanced `{...}` JSON object from a string.
/// Uses brace-depth counting so nested objects and trailing prose don't confuse the extractor.
fn extract_json_object(s: &str) -> Option<&str> {
    let start = s.find('{')?;
    let mut depth = 0usize;
    for (i, ch) in s[start..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&s[start..=start + i]);
                }
            }
            _ => {}
        }
    }
    None // unbalanced braces
}

/// Try to parse a voice command locally using keyword/regex patterns.
/// Returns None if the command is ambiguous and needs Gemini.
pub fn parse_local_intent(text: &str) -> Option<MusicIntent> {
    let lower = text.to_lowercase().trim().to_string();

    // Transport controls (no query needed)
    let transport = match lower.as_str() {
        s if s.contains("pause") || s.contains("pausa") => Some(IntentAction::Pause),
        s if s.contains("resume") || s.contains("continua") || s.contains("reanudar") => {
            Some(IntentAction::Resume)
        }
        s if s == "next" || s == "skip" || s.contains("next song") || s.contains("siguiente") => {
            Some(IntentAction::Next)
        }
        s if s == "previous"
            || s == "prev"
            || s.contains("previous song")
            || s.contains("anterior") =>
        {
            Some(IntentAction::Previous)
        }
        s if s == "stop" || s == "para" || s == "detener" => Some(IntentAction::Stop),
        s if s.contains("volume up") || s.contains("sube") || s.contains("louder") => {
            Some(IntentAction::VolumeUp)
        }
        s if s.contains("volume down") || s.contains("baja") || s.contains("quieter") => {
            Some(IntentAction::VolumeDown)
        }
        s if s == "shuffle" || s.contains("aleatorio") => Some(IntentAction::Shuffle),
        s if s.contains("clear queue")
            || s.contains("clear the queue")
            || s.contains("limpiar")
            || s.contains("vaciar") =>
        {
            Some(IntentAction::ClearQueue)
        }
        _ => None,
    };

    if let Some(action) = transport {
        return Some(MusicIntent {
            action,
            query: None,
        });
    }

    // Content-based commands with simple patterns
    // "play <something>" / "pon <algo>"
    if let Some(rest) = strip_prefix_any(&lower, &["play ", "pon ", "reproduce "]) {
        return Some(MusicIntent {
            action: IntentAction::Play,
            query: Some(IntentQuery {
                free_text: Some(rest.to_string()),
                ..Default::default()
            }),
        });
    }

    // "queue <something>" / "encola <algo>" / "add <something>"
    if let Some(rest) = strip_prefix_any(&lower, &["queue ", "encola ", "agregar ", "add "]) {
        return Some(MusicIntent {
            action: IntentAction::Queue,
            query: Some(IntentQuery {
                free_text: Some(rest.to_string()),
                ..Default::default()
            }),
        });
    }

    // "search <something>" / "busca <algo>"
    if let Some(rest) = strip_prefix_any(&lower, &["search ", "busca ", "buscar "]) {
        return Some(MusicIntent {
            action: IntentAction::Search,
            query: Some(IntentQuery {
                free_text: Some(rest.to_string()),
                ..Default::default()
            }),
        });
    }

    None // Ambiguous — needs Gemini
}

fn strip_prefix_any<'a>(s: &'a str, prefixes: &[&str]) -> Option<&'a str> {
    for prefix in prefixes {
        if let Some(rest) = s.strip_prefix(prefix) {
            let trimmed = rest.trim();
            if !trimmed.is_empty() {
                return Some(trimmed);
            }
        }
    }
    None
}

/// Human-readable description of a music intent.
pub fn describe_intent(intent: &MusicIntent) -> String {
    match intent.action {
        IntentAction::Play => {
            if let Some(ref q) = intent.query {
                let mut parts = Vec::new();
                if let Some(ref t) = q.title {
                    parts.push(format!("\"{}\"", t));
                }
                if let Some(ref a) = q.artist {
                    parts.push(format!("by {}", a));
                }
                if let Some(ref al) = q.album {
                    parts.push(format!("album: {}", al));
                }
                if let Some(ref g) = q.genre {
                    parts.push(format!("{} music", g));
                }
                if let Some(ref m) = q.mood {
                    parts.push(format!("{} vibes", m));
                }
                if let Some(ref ft) = q.free_text {
                    parts.push(ft.clone());
                }
                format!("Playing {}", parts.join(" "))
            } else {
                "Playing music...".into()
            }
        }
        IntentAction::Queue => {
            if let Some(ref q) = intent.query {
                let mut parts = Vec::new();
                if let Some(ref a) = q.artist {
                    parts.push(a.clone());
                }
                if let Some(ref t) = q.title {
                    parts.push(format!("\"{}\"", t));
                }
                if let Some(ref g) = q.genre {
                    parts.push(format!("{} music", g));
                }
                if let Some(ref ft) = q.free_text {
                    parts.push(ft.clone());
                }
                format!("Queuing {}", parts.join(", "))
            } else {
                "Adding to queue...".into()
            }
        }
        IntentAction::Search => {
            if let Some(ref q) = intent.query {
                let mut parts = Vec::new();
                if let Some(ref a) = q.artist {
                    parts.push(a.clone());
                }
                if let Some(ref t) = q.title {
                    parts.push(t.clone());
                }
                if let Some(ref g) = q.genre {
                    parts.push(g.clone());
                }
                if let Some(ref ft) = q.free_text {
                    parts.push(ft.clone());
                }
                format!("Searching for {}", parts.join(", "))
            } else {
                "Searching...".into()
            }
        }
        IntentAction::Pause => "Paused".into(),
        IntentAction::Resume => "Playing".into(),
        IntentAction::Next => "Next track".into(),
        IntentAction::Previous => "Previous track".into(),
        IntentAction::Stop => "Stopped".into(),
        IntentAction::VolumeUp => "Volume up".into(),
        IntentAction::VolumeDown => "Volume down".into(),
        IntentAction::Shuffle => "Shuffle toggled".into(),
        IntentAction::ClearQueue => "Queue cleared".into(),
    }
}
