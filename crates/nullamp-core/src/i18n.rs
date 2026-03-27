use std::collections::HashMap;

/// Build the full translation table for all supported languages.
pub fn translations() -> HashMap<&'static str, HashMap<&'static str, &'static str>> {
    let mut all = HashMap::new();
    all.insert("en", en());
    all.insert("es", es());
    all
}

/// Get a translation for a key in the given language, falling back to English.
pub fn t<'a>(
    table: &'a HashMap<&str, HashMap<&str, &'a str>>,
    lang: &str,
    key: &'a str,
) -> &'a str {
    table
        .get(lang)
        .and_then(|m| m.get(key).copied())
        .or_else(|| table.get("en").and_then(|m| m.get(key).copied()))
        .unwrap_or(key)
}

fn en() -> HashMap<&'static str, &'static str> {
    let mut m = HashMap::new();
    // Navigation tabs
    m.insert("tab_library", "Library");
    m.insert("tab_search", "Search");
    m.insert("tab_voice", "Voice");
    m.insert("tab_eq", "EQ");

    // Queue panel
    m.insert("queue", "Queue");
    m.insert("queue_empty", "Queue is empty");
    m.insert("queue_hint", "Right-click tracks in Library to add");

    // Library browser
    m.insert("scanning", "Scanning library...");
    m.insert("no_library", "No music library configured.");
    m.insert("no_library_hint", "Open Settings to add a music folder.");
    m.insert("no_artist_meta", "No artist metadata found.");
    m.insert("no_artist_hint", "Use the \"All\" view to see all tracks.");
    m.insert(
        "double_click_hint",
        "Double-click to play \u{00b7} Right-click for options",
    );
    m.insert("queue_all", "+ Queue All");
    m.insert("view_all", "All");
    m.insert("view_artists", "Artists");

    // Search
    m.insert("search_placeholder", "Search tracks, artists, albums...");
    m.insert("search_type_hint", "Type to search your library");
    m.insert("search_no_results", "No results for");
    m.insert("result", "result");
    m.insert("results", "results");

    // Context menus
    m.insert("play_now", "Play Now");
    m.insert("play_next", "Play Next");
    m.insert("add_to_queue", "Add to Queue");
    m.insert("remove_from_queue", "Remove from Queue");

    // Voice panel
    m.insert("voice_click_to_speak", "Click to speak a command");
    m.insert("voice_listening", "Listening\u{2026} Click to stop");
    m.insert("voice_transcribing", "Transcribing\u{2026}");
    m.insert("voice_thinking", "Understanding intent\u{2026}");
    m.insert("voice_i_heard", "I heard:");
    m.insert("voice_intent", "Intent:");
    m.insert("voice_example_1", "\"Play all Queen music\"");
    m.insert("voice_example_2", "\"Queue some jazz\"");
    m.insert("voice_example_3", "\"Next song\"");
    m.insert("voice_model_required", "Speech Recognition Model");
    m.insert(
        "voice_model_desc",
        "Download a model to enable voice commands",
    );
    m.insert("voice_download", "Download");
    m.insert("voice_type_command", "or type a command\u{2026}");
    m.insert("voice_change_model", "Change model");
    m.insert("voice_model_speed", "Speed");
    m.insert("voice_model_accuracy", "Accuracy");
    m.insert("voice_model_active", "active");
    m.insert("voice_model_in_use", "In use");
    m.insert("voice_model_use", "Use");
    m.insert("voice_model_back", "\u{2190} Back");

    // Equalizer
    m.insert("eq_on", "ON");
    m.insert("eq_presets", "PRESETS \u{25be}");
    m.insert("eq_custom", "Custom");

    // Settings
    m.insert("settings", "Settings");
    m.insert("settings_desc", "Configure Nullamp preferences");
    m.insert("api_key", "Gemini API Key");
    m.insert("gemini_model", "Gemini Model");
    m.insert("lang_english", "English");
    m.insert("lang_spanish", "Spanish");
    m.insert("music_folder", "Music Folder");
    m.insert("select_folder", "Select Folder");
    m.insert("change_folder", "Change Folder");
    m.insert("save", "Save");
    m.insert("not_set", "not set");
    m.insert("configured", "configured");

    // Settings buttons
    m.insert("browse", "Browse\u{2026}");
    m.insert("scan_library", "Scan Library");
    m.insert("show", "Show");
    m.insert("hide", "Hide");
    m.insert("close", "Close");
    m.insert("language", "Language");
    m.insert("whisper_models", "Whisper Models");

    // Player / queue
    m.insert("no_track_loaded", "No track loaded");
    m.insert("tracks", "tracks");

    // Status bar
    m.insert("tracks_in_library", "tracks in library");

    // Scan progress
    m.insert("scan_discovering", "Discovering files\u{2026}");
    m.insert("scan_indexing", "Indexing");
    m.insert("scan_removing", "Cleaning up\u{2026}");
    m.insert("scan_complete", "Scan complete");
    m.insert("scan_cancelled", "Scan cancelled");
    m.insert("scan_cancel", "Cancel");
    m.insert("scan_files_per_sec", "f/s");
    m.insert("scan_eta", "ETA");
    m.insert("scan_new", "new");
    m.insert("scan_removed", "removed");

    m
}

fn es() -> HashMap<&'static str, &'static str> {
    let mut m = HashMap::new();
    m.insert("tab_library", "Biblioteca");
    m.insert("tab_search", "Buscar");
    m.insert("tab_voice", "Voz");
    m.insert("tab_eq", "EQ");

    m.insert("queue", "Cola");
    m.insert("queue_empty", "La cola est\u{00e1} vac\u{00ed}a");
    m.insert("queue_hint", "Clic derecho en una pista para agregar");

    m.insert("scanning", "Escaneando biblioteca\u{2026}");
    m.insert(
        "no_library",
        "No hay biblioteca de m\u{00fa}sica configurada.",
    );
    m.insert(
        "no_library_hint",
        "Abre Ajustes para agregar una carpeta de m\u{00fa}sica.",
    );
    m.insert(
        "no_artist_meta",
        "No se encontraron artistas con metadatos.",
    );
    m.insert(
        "no_artist_hint",
        "Usa la vista \"Todo\" para ver todas las pistas.",
    );
    m.insert(
        "double_click_hint",
        "Doble clic para reproducir \u{00b7} Clic derecho para opciones",
    );
    m.insert("queue_all", "+ Agregar todo");
    m.insert("view_all", "Todo");
    m.insert("view_artists", "Artistas");

    m.insert(
        "search_placeholder",
        "Buscar pistas, artistas, \u{00e1}lbumes\u{2026}",
    );
    m.insert("search_type_hint", "Escribe para buscar en tu biblioteca");
    m.insert("search_no_results", "Sin resultados para");
    m.insert("result", "resultado");
    m.insert("results", "resultados");

    m.insert("play_now", "Reproducir ahora");
    m.insert("play_next", "Reproducir despu\u{00e9}s");
    m.insert("add_to_queue", "Agregar a la cola");
    m.insert("remove_from_queue", "Quitar de la cola");

    m.insert("voice_click_to_speak", "Haz clic para dar un comando");
    m.insert(
        "voice_listening",
        "Escuchando\u{2026} Haz clic para detener",
    );
    m.insert("voice_transcribing", "Transcribiendo\u{2026}");
    m.insert("voice_thinking", "Entendiendo la intenci\u{00f3}n\u{2026}");
    m.insert("voice_i_heard", "Escuch\u{00e9}:");
    m.insert("voice_intent", "Intenci\u{00f3}n:");
    m.insert("voice_example_1", "\"Pon m\u{00fa}sica de Queen\"");
    m.insert("voice_example_2", "\"Agrega jazz a la cola\"");
    m.insert("voice_example_3", "\"Siguiente canci\u{00f3}n\"");
    m.insert("voice_model_required", "Modelo de Reconocimiento de Voz");
    m.insert(
        "voice_model_desc",
        "Descarga un modelo para activar los comandos de voz",
    );
    m.insert("voice_download", "Descargar");
    m.insert("voice_type_command", "o escribe un comando\u{2026}");
    m.insert("voice_change_model", "Cambiar modelo");
    m.insert("voice_model_speed", "Velocidad");
    m.insert("voice_model_accuracy", "Precisi\u{00f3}n");
    m.insert("voice_model_active", "activo");
    m.insert("voice_model_in_use", "En uso");
    m.insert("voice_model_use", "Usar");
    m.insert("voice_model_back", "\u{2190} Volver");

    m.insert("eq_on", "ON");
    m.insert("eq_presets", "PRESETS \u{25be}");
    m.insert("eq_custom", "Custom");

    m.insert("settings", "Ajustes");
    m.insert("settings_desc", "Configura las preferencias de Nullamp");
    m.insert("api_key", "Clave API de Gemini");
    m.insert("gemini_model", "Modelo Gemini");
    m.insert("lang_english", "Ingl\u{00e9}s");
    m.insert("lang_spanish", "Espa\u{00f1}ol");
    m.insert("music_folder", "Carpeta de m\u{00fa}sica");
    m.insert("select_folder", "Seleccionar carpeta");
    m.insert("change_folder", "Cambiar carpeta");
    m.insert("save", "Guardar");
    m.insert("not_set", "no configurado");
    m.insert("configured", "configurado");

    // Settings buttons
    m.insert("browse", "Examinar\u{2026}");
    m.insert("scan_library", "Escanear biblioteca");
    m.insert("show", "Mostrar");
    m.insert("hide", "Ocultar");
    m.insert("close", "Cerrar");
    m.insert("language", "Idioma");
    m.insert("whisper_models", "Modelos Whisper");

    // Player / queue
    m.insert("no_track_loaded", "Sin pista cargada");
    m.insert("tracks", "pistas");

    m.insert("tracks_in_library", "pistas en biblioteca");

    m.insert("scan_discovering", "Descubriendo archivos\u{2026}");
    m.insert("scan_indexing", "Indexando");
    m.insert("scan_removing", "Limpiando\u{2026}");
    m.insert("scan_complete", "Escaneo completo");
    m.insert("scan_cancelled", "Escaneo cancelado");
    m.insert("scan_cancel", "Cancelar");
    m.insert("scan_files_per_sec", "a/s");
    m.insert("scan_eta", "ETA");
    m.insert("scan_new", "nuevos");
    m.insert("scan_removed", "eliminados");

    m
}
