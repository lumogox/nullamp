use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use rodio::Source;

/// Center frequencies for the 10-band EQ (Hz).
pub const EQ_FREQUENCIES: [f32; 10] = [
    60.0, 170.0, 310.0, 600.0, 1000.0, 3000.0, 6000.0, 12000.0, 14000.0, 16000.0,
];

/// Q factor for peaking EQ filters (matches Web Audio BiquadFilterNode default).
const Q: f32 = 1.4;

/// Named EQ presets — each is 10 gain values in dB.
pub const EQ_PRESETS: &[(&str, [f32; 10])] = &[
    ("Flat", [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]),
    (
        "Classical",
        [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -4.0, -4.0, -4.0, -6.0],
    ),
    ("Club", [0.0, 0.0, 4.0, 5.0, 5.0, 3.0, 0.0, 0.0, 0.0, 0.0]),
    (
        "Dance",
        [6.0, 4.0, 1.0, 0.0, 0.0, -3.0, -4.0, -4.0, 0.0, 0.0],
    ),
    (
        "Full Bass",
        [5.0, 5.0, 5.0, 3.0, 1.0, -4.0, -6.0, -7.0, -7.0, -7.0],
    ),
    (
        "Full Treble",
        [-7.0, -7.0, -7.0, -3.0, 1.0, 6.0, 9.0, 9.0, 9.0, 10.0],
    ),
    (
        "Full Bass & Treble",
        [4.0, 3.0, 0.0, -4.0, -3.0, 1.0, 4.0, 6.0, 7.0, 7.0],
    ),
    (
        "Laptop",
        [3.0, 7.0, 9.0, 4.0, 0.0, -1.0, -2.0, -3.0, -3.0, -1.0],
    ),
    (
        "Large Hall",
        [6.0, 6.0, 3.0, 3.0, 0.0, -2.0, -2.0, -2.0, 0.0, 0.0],
    ),
    ("Live", [-3.0, 0.0, 2.0, 3.0, 3.0, 3.0, 2.0, 1.0, 1.0, 1.0]),
    ("Party", [5.0, 4.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 4.0, 5.0]),
    (
        "Pop",
        [-1.0, 3.0, 4.0, 5.0, 3.0, 0.0, -1.0, -1.0, -1.0, -1.0],
    ),
    (
        "Reggae",
        [0.0, 0.0, 0.0, -3.0, 0.0, 4.0, 4.0, 0.0, 0.0, 0.0],
    ),
    (
        "Rock",
        [5.0, 3.0, -3.0, -4.0, -2.0, 2.0, 5.0, 6.0, 6.0, 6.0],
    ),
    ("Ska", [-1.0, -3.0, -3.0, 0.0, 2.0, 3.0, 5.0, 5.0, 6.0, 6.0]),
    ("Soft", [3.0, 1.0, 0.0, -1.0, 0.0, 2.0, 5.0, 6.0, 7.0, 7.0]),
    (
        "Soft Rock",
        [2.0, 1.0, 0.0, -1.0, -2.0, 0.0, 2.0, 5.0, 7.0, 8.0],
    ),
    (
        "Techno",
        [5.0, 3.0, 0.0, -3.0, -3.0, 0.0, 5.0, 6.0, 6.0, 5.0],
    ),
];

// ─── Shared atomic parameters (UI thread writes, audio thread reads) ───

fn f32_to_bits(v: f32) -> u32 {
    v.to_bits()
}

fn bits_to_f32(v: u32) -> f32 {
    f32::from_bits(v)
}

/// Shared EQ parameters updated atomically from the UI thread.
pub struct EqParams {
    /// Per-band gain in dB, stored as atomic u32 (f32 bit pattern).
    band_gains: [AtomicU32; 10],
    /// Preamp gain in dB.
    preamp_db: AtomicU32,
    /// Whether EQ processing is enabled.
    enabled: AtomicU32, // 0 = disabled, 1 = enabled
}

impl EqParams {
    pub fn new() -> Self {
        Self {
            band_gains: std::array::from_fn(|_| AtomicU32::new(f32_to_bits(0.0))),
            preamp_db: AtomicU32::new(f32_to_bits(0.0)),
            enabled: AtomicU32::new(1),
        }
    }

    pub fn set_band(&self, index: usize, gain_db: f32) {
        if index < 10 {
            self.band_gains[index].store(f32_to_bits(gain_db), Ordering::Relaxed);
        }
    }

    pub fn get_band(&self, index: usize) -> f32 {
        if index < 10 {
            bits_to_f32(self.band_gains[index].load(Ordering::Relaxed))
        } else {
            0.0
        }
    }

    pub fn set_preamp(&self, db: f32) {
        self.preamp_db.store(f32_to_bits(db), Ordering::Relaxed);
    }

    pub fn get_preamp(&self) -> f32 {
        bits_to_f32(self.preamp_db.load(Ordering::Relaxed))
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.enabled
            .store(if enabled { 1 } else { 0 }, Ordering::Relaxed);
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed) != 0
    }

    /// Load a preset by name, setting all 10 bands and resetting preamp.
    pub fn load_preset(&self, name: &str) -> bool {
        if let Some((_, bands)) = EQ_PRESETS.iter().find(|(n, _)| *n == name) {
            for (i, &g) in bands.iter().enumerate() {
                self.set_band(i, g);
            }
            self.set_preamp(0.0);
            true
        } else {
            false
        }
    }
}

impl Default for EqParams {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Biquad filter (second-order IIR, peaking EQ) ───

/// Coefficients for a normalized biquad filter: y[n] = b0*x[n] + b1*x[n-1] + b2*x[n-2] - a1*y[n-1] - a2*y[n-2]
#[derive(Clone, Copy)]
struct BiquadCoeffs {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
}

impl BiquadCoeffs {
    /// Compute peaking EQ coefficients using the Audio EQ Cookbook formula.
    fn peaking(freq: f32, gain_db: f32, q: f32, sample_rate: f32) -> Self {
        if gain_db.abs() < 0.001 {
            // Unity (bypass)
            return Self {
                b0: 1.0,
                b1: 0.0,
                b2: 0.0,
                a1: 0.0,
                a2: 0.0,
            };
        }

        let a = 10.0_f32.powf(gain_db / 40.0); // sqrt(10^(dB/20))
        let w0 = 2.0 * std::f32::consts::PI * freq / sample_rate;
        let sin_w0 = w0.sin();
        let cos_w0 = w0.cos();
        let alpha = sin_w0 / (2.0 * q);

        let a0 = 1.0 + alpha / a;
        Self {
            b0: (1.0 + alpha * a) / a0,
            b1: (-2.0 * cos_w0) / a0,
            b2: (1.0 - alpha * a) / a0,
            a1: (-2.0 * cos_w0) / a0,
            a2: (1.0 - alpha / a) / a0,
        }
    }
}

/// Per-channel state for a single biquad filter section.
#[derive(Clone, Copy, Default)]
struct BiquadState {
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
}

impl BiquadState {
    fn process(&mut self, input: f32, c: &BiquadCoeffs) -> f32 {
        let output =
            c.b0 * input + c.b1 * self.x1 + c.b2 * self.x2 - c.a1 * self.y1 - c.a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = input;
        self.y2 = self.y1;
        self.y1 = output;
        output
    }
}

// ─── EQ Source wrapper ───

/// A rodio `Source` adapter that applies a 10-band parametric EQ.
/// Band gains are read from shared `EqParams` atomics each time coefficients
/// are recomputed (every `RECALC_INTERVAL` samples to avoid per-sample atomic reads).
pub struct EqSource<S> {
    inner: S,
    params: Arc<EqParams>,
    sample_rate: u32,
    channels: u16,
    /// Current biquad coefficients for each band.
    coeffs: [BiquadCoeffs; 10],
    /// Per-channel, per-band filter state. Indexed [channel][band].
    states: Vec<[BiquadState; 10]>,
    /// Current preamp linear gain.
    preamp_linear: f32,
    /// Whether EQ is enabled (cached from params).
    enabled: bool,
    /// Counter for periodic coefficient recalculation.
    sample_counter: u32,
}

/// Recalculate coefficients every N samples (~23ms at 44.1kHz).
const RECALC_INTERVAL: u32 = 1024;

impl<S> EqSource<S>
where
    S: Source<Item = f32>,
{
    pub fn new(inner: S, params: Arc<EqParams>) -> Self {
        let sample_rate = inner.sample_rate();
        let channels = inner.channels();

        let mut source = Self {
            inner,
            params,
            sample_rate,
            channels,
            coeffs: [BiquadCoeffs {
                b0: 1.0,
                b1: 0.0,
                b2: 0.0,
                a1: 0.0,
                a2: 0.0,
            }; 10],
            states: vec![[BiquadState::default(); 10]; channels as usize],
            preamp_linear: 1.0,
            enabled: true,
            sample_counter: 0,
        };
        source.recalc_coefficients();
        source
    }

    fn recalc_coefficients(&mut self) {
        self.enabled = self.params.is_enabled();
        if !self.enabled {
            return;
        }

        let preamp_db = self.params.get_preamp();
        self.preamp_linear = 10.0_f32.powf(preamp_db / 20.0);

        for (i, &freq) in EQ_FREQUENCIES.iter().enumerate() {
            let gain_db = self.params.get_band(i);
            self.coeffs[i] = BiquadCoeffs::peaking(freq, gain_db, Q, self.sample_rate as f32);
        }
    }
}

impl<S> Iterator for EqSource<S>
where
    S: Source<Item = f32>,
{
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        let sample = self.inner.next()?;

        // Periodically re-read shared params (amortized atomic reads)
        if self.sample_counter == 0 {
            self.recalc_coefficients();
        }
        self.sample_counter = (self.sample_counter + 1) % RECALC_INTERVAL;

        if !self.enabled {
            return Some(sample);
        }

        // Determine which channel this sample belongs to
        let ch = (self.sample_counter as usize) % self.channels as usize;

        // Apply preamp
        let mut s = sample * self.preamp_linear;

        // Cascade through 10 biquad filters
        for band in 0..10 {
            s = self.states[ch][band].process(s, &self.coeffs[band]);
        }

        Some(s)
    }
}

impl<S> Source for EqSource<S>
where
    S: Source<Item = f32>,
{
    fn current_frame_len(&self) -> Option<usize> {
        self.inner.current_frame_len()
    }

    fn channels(&self) -> u16 {
        self.inner.channels()
    }

    fn sample_rate(&self) -> u32 {
        self.inner.sample_rate()
    }

    fn total_duration(&self) -> Option<Duration> {
        self.inner.total_duration()
    }
}
