use std::path::{Path, PathBuf};

use bvc_client_lib::testkit::signal::Signal;

/// Bm progression note set (B3, D4, F#4). Asserted present for jukebox A.
pub const BM_NOTES: [f32; 3] = [246.94, 293.66, 369.99];
/// High A-major progression note set (A5, C#6, E6). Asserted present for jukebox
/// B. Deliberately a high register, well separated from BM_NOTES, because the
/// single-bin Goertzel leaks heavily between near frequencies — separation keeps
/// the cross-bleed assertion (one set present, the other absent) unambiguous.
pub const A_NOTES: [f32; 3] = [880.00, 1108.73, 1318.51];

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

    /// Builds the A-major progression (the "different" one) for the concurrent case.
    pub fn a_major_wav(dir: &Path, name: &str, repeats: u32) -> PathBuf {
        let pcm = Signal::progression(&A_NOTES, &A_NOTES, 0.4, 0.6, repeats, 48_000);
        let path = dir.join(format!("{name}.wav"));
        Self::write_wav_48k_mono(&pcm, &path);
        path
    }
}
