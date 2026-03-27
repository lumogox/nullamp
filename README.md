# NULLAMP

A **Winamp 2.x-inspired AI voice-controlled music player** built in pure Rust. Voice commands, full-text search, keyboard shortcuts, 10-band equalizer, and instant library indexing across 280k+ files.

![Voice Command Panel](https://github.com/user-attachments/assets/37b49d3d-7141-4d02-b2e5-af69109419da)
![Equalizer & Playlist](https://github.com/user-attachments/assets/ef154018-61f2-4b59-995b-fabd1cd3e521)

## Key Features

### Voice Commands
- **Local speech-to-text**: Whisper (ggml) — English & Spanish, no cloud
- **Intent extraction**: Google Gemini API with fallback to local regex patterns
- **Natural language**: "Play some jazz", "Queue Queen", "Next song"

### Library & Search
- **Parallel indexing**: Rayon + batched SQLite writes — 280k files in ~12 minutes
- **Multi-format**: MP3, FLAC, OGG, M4A, WAV, AIFF, WMA, Opus, APE, WV
- **Full-text search**: Artists, albums, tracks with instant results
- **Incremental scans**: Only re-index changed files on subsequent runs

### 10-Band Equalizer
- Frequency bands: 31Hz–16kHz
- 7 built-in presets (bass boost, treble, rock, pop, jazz, classical)
- ±12dB per band, real-time BiquadFilter applied to playback

### Retro Aesthetic
- Winamp 2.x dark theme — monospace font, green + amber accents
- Frameless window with custom draggable title bar

### Keyboard Shortcuts
- **Playback**: Space (play/pause), N (next), P (prev)
- **Volume**: Arrow keys Up/Down
- **Seek**: Arrow keys Left/Right

---

## Tech Stack

| Layer | Library |
|-------|---------|
| UI framework | [Iced](https://github.com/iced-rs/iced) 0.13 (wgpu renderer) |
| Audio playback | [rodio](https://github.com/RustAudio/rodio) 0.20 |
| Mic capture | [cpal](https://github.com/RustAudio/cpal) 0.15 |
| Speech recognition | Whisper.cpp (ggml) via whisper-rs |
| AI intent | Google Gemini API (optional) |
| Library DB | SQLite via rusqlite |
| Metadata | lofty 0.22 |
| Parallel indexing | Rayon |

---

## Building from Source

### Prerequisites

- **Rust** 1.75+ — install via [rustup](https://rustup.rs)
- **Git**

No Node.js, no npm, no Tauri CLI required.

---

### macOS (Apple Silicon)

```bash
# Install Xcode command line tools if not present
xcode-select --install

# Clone and build
git clone https://github.com/yourusername/nullamp.git
cd nullamp
cargo build --release --package nullamp-app

# Run
./target/release/nullamp
```

> **Intel Mac**: Replace the build command with:
> `cargo build --release --package nullamp-app --target x86_64-apple-darwin`
> (requires `rustup target add x86_64-apple-darwin`)

---

### Windows (x64)

Open **Developer PowerShell** or a regular terminal with Rust installed.

```powershell
# Clone and build
git clone https://github.com/yourusername/nullamp.git
cd nullamp
cargo build --release --package nullamp-app

# Run
.\target\release\nullamp.exe
```

The MSVC toolchain is used automatically on Windows. No extra system libraries needed.

---

### Linux (x64)

Install system dependencies first (for audio via ALSA and GPU rendering via wgpu/Vulkan):

**Debian / Ubuntu:**
```bash
sudo apt-get update
sudo apt-get install -y \
  libasound2-dev \
  libxkbcommon-dev \
  libgl1-mesa-dev \
  libx11-dev \
  libxcursor-dev \
  libxrandr-dev \
  libxi-dev \
  libwayland-dev \
  libfontconfig1-dev \
  pkg-config
```

**Fedora / RHEL:**
```bash
sudo dnf install -y \
  alsa-lib-devel \
  libxkbcommon-devel \
  mesa-libGL-devel \
  libX11-devel \
  libXcursor-devel \
  libXrandr-devel \
  libXi-devel \
  wayland-devel \
  fontconfig-devel \
  pkg-config
```

**Arch Linux:**
```bash
sudo pacman -S --needed \
  alsa-lib \
  libxkbcommon \
  mesa \
  libx11 \
  libxcursor \
  libxrandr \
  libxi \
  wayland \
  fontconfig \
  pkgconf
```

Then build and run:
```bash
git clone https://github.com/yourusername/nullamp.git
cd nullamp
cargo build --release --package nullamp-app

./target/release/nullamp
```

---

## Configuration

1. **Music Folder** — click the folder icon in Settings, point to your library. Scan starts automatically.
2. **Gemini API Key** (optional) — get a free key at [aistudio.google.com/apikey](https://aistudio.google.com/apikey), enter in Settings. Only used for ambiguous voice commands; local keyword parser handles common commands.
3. **Language** — English (`en`) or Spanish (`es`), toggle in the title bar or Settings.

---

## Voice Model Management

Open the **Voice** tab to download Whisper models in-app:

| Model | Size | Speed | Accuracy |
|-------|------|-------|----------|
| Tiny | 75 MB | Fastest | Lower |
| Base | 142 MB | Fast | Balanced |
| Small | 466 MB | Medium | Good |
| Medium | 1.5 GB | Slow | High |

Models are cached in `~/.nullamp/models/`. Switch anytime from the Voice tab.

---

## Voice Command Examples

| Command | Action |
|---------|--------|
| "Play all Queen music" | Search, queue all, play |
| "Queue some jazz" | Search, add to queue |
| "Next song" | Skip to next track |
| "Pause" | Pause playback |
| "Volume up" | Increase volume |
| "Search rock" | Filter library |

---

## Project Structure

```
nullamp/
├── crates/
│   ├── nullamp-core/     # DB, models, indexer, whisper, intent, i18n, settings
│   ├── nullamp-audio/    # rodio player, biquad EQ, cpal mic capture
│   └── nullamp-app/      # Iced UI (main.rs, app.rs, theme.rs, message.rs, views/)
│       └── src/views/
│           ├── player_bar.rs   # Recessed display, seek, transport, volume
│           ├── library.rs      # Track list, search
│           ├── equalizer.rs    # 10-band EQ with curve canvas
│           ├── voice.rs        # Mic button, model grid, waveform
│           ├── queue.rs        # Queue panel
│           └── settings.rs     # Folder picker, API key, language
└── Cargo.toml
```

---

## Performance

| Scenario | Time | Notes |
|----------|------|-------|
| First scan (280k files) | ~12 min | Parallel extraction + batched DB writes |
| Incremental rescan (0 changes) | <1 sec | Discovery + HashSet filter only |
| Text search | <50ms | Full-text SQLite index |
| Voice intent (local) | ~5ms | Keyword/regex matching |
| Voice intent (Gemini) | 1–3 sec | Natural language fallback |

---

## License

MIT — feel free to fork, modify, and use this project.

---

## Credits

- **OpenAI Whisper** — speech recognition model (ggml format)
- **Google Gemini** — intent disambiguation
- **Iced** — Rust-native GUI framework
- **lofty** — audio metadata library
- **rodio / cpal** — cross-platform audio

---

**Built with love by Luis**
