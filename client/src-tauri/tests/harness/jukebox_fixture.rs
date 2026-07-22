use std::path::{Path, PathBuf};

use bvc_client_lib::testkit::signal::Signal;

/// Bm progression note set (B3, D4, F#4). Asserted present for jukebox A.
pub const BM_NOTES: [f32; 3] = [246.94, 293.66, 369.99];
/// High C-major progression note set (C6, E6, G6). Asserted present for jukebox
/// B. Two constraints picked these: a high register well separated from
/// BM_NOTES (the single-bin Goertzel leaks heavily between near frequencies),
/// and — critically — no bin may sit on a HARMONIC of the other scale's notes.
/// The previous A-major set failed that: A5 (880.00) is 3×D4 (880.98) and C#6
/// (1108.73) is 3×F#4 (1109.97), so codec/pipeline nonlinearity put the Bm
/// track's own 3rd-harmonic energy into the "absent" bins, right at the 1%
/// threshold (flaky). A is the fifth of D and C# the fifth of F#, so every A/C#
/// octave collides; C-E-G clears every Bm harmonic through the 8th by ≥58 Hz.
pub const C_NOTES: [f32; 3] = [1046.50, 1318.51, 1567.98];

pub struct JukeboxFixture;

impl JukeboxFixture {
    /// Writes a 48 kHz mono 16-bit PCM WAV at `path` from f32 samples in [-1, 1].
    /// 48 kHz mono matches the Opus encode rate so the upload path performs no
    /// resampling, keeping the note frequencies intact for Goertzel assertions.
    pub fn write_wav_48k_mono(samples: &[f32], path: &Path) {
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 48_000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(path, spec).expect("create wav writer");
        for &s in samples {
            let v = (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
            writer.write_sample(v).expect("write sample");
        }
        writer.finalize().expect("finalize wav");
    }

    /// Builds the standard Bm progression (sequence + chord), `repeats` loops, and
    /// writes it to `dir/<name>.wav`, returning the path.
    pub fn bm_wav(dir: &Path, name: &str, repeats: u32) -> PathBuf {
        let pcm = Signal::progression(&BM_NOTES, &BM_NOTES, 0.4, 0.6, repeats, 48_000);
        let path = dir.join(format!("{name}.wav"));
        Self::write_wav_48k_mono(&pcm, &path);
        path
    }

    /// Builds the C-major progression (the "different" one) for the concurrent case.
    pub fn c_major_wav(dir: &Path, name: &str, repeats: u32) -> PathBuf {
        let pcm = Signal::progression(&C_NOTES, &C_NOTES, 0.4, 0.6, repeats, 48_000);
        let path = dir.join(format!("{name}.wav"));
        Self::write_wav_48k_mono(&pcm, &path);
        path
    }
}
