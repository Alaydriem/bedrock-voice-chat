// Pure DSP helpers for the e2e harness's orchestrator side: synthesize probe
// signals and quantify how much of one buffer survives in another after a round
// trip through capture/encode/decode/mix. All methods are associated functions
// on the unit struct `Signal` to honor the crate's structs-over-free-functions
// policy. Nothing here touches device IO; the math is exact for test sizes.

use std::f32::consts::PI;

// C-major triad frequencies used by `musical_probe`.
const NOTE_C4: f32 = 261.63;
const NOTE_E4: f32 = 329.63;
const NOTE_G4: f32 = 392.00;

pub struct Signal;

impl Signal {
    // Musical probe signal: a 1-3-5-3-1 melodic run in C major followed by a
    // C-major chord, totalling ~3.0 s. The distinct notes (C4, E4, G4) let tests
    // assert that specific frequencies survived the Opus round-trip, which a plain
    // chirp cannot differentiate.
    //
    // Structure:
    //   0.0 – 2.0 s: five 0.4 s melody segments — C4, E4, G4, E4, C4 — each a
    //                pure sine at amplitude 0.4, with a 5 ms linear fade-in/out
    //                applied to the first and last 5 ms of each segment to
    //                suppress click transients at note boundaries.
    //   2.0 – 3.0 s: C4 + E4 + G4 summed simultaneously, each at amplitude 0.20
    //                (combined peak ≤ 0.60; 0.60 × 1.3 channel boost = 0.78, no clip).
    pub fn musical_probe(sample_rate: u32) -> Vec<f32> {
        let sr = sample_rate as f32;
        let fade_samples = ((sr * 0.005).round() as usize).max(1);
        let seg_samples = (sr * 0.4).round() as usize;
        let chord_samples = (sr * 1.0).round() as usize;
        let total = seg_samples * 5 + chord_samples;
        let mut out = Vec::with_capacity(total);

        let melody_notes = [NOTE_C4, NOTE_E4, NOTE_G4, NOTE_E4, NOTE_C4];

        for &freq in &melody_notes {
            for i in 0..seg_samples {
                let t = i as f32 / sr;
                let mut amp = 0.4_f32;
                if i < fade_samples {
                    amp *= i as f32 / fade_samples as f32;
                } else if i >= seg_samples - fade_samples {
                    amp *= (seg_samples - 1 - i) as f32 / fade_samples as f32;
                }
                out.push(amp * (2.0 * PI * freq * t).sin());
            }
        }

        for i in 0..chord_samples {
            let t = i as f32 / sr;
            let s = 0.20 * (2.0 * PI * NOTE_C4 * t).sin()
                + 0.20 * (2.0 * PI * NOTE_E4 * t).sin()
                + 0.20 * (2.0 * PI * NOTE_G4 * t).sin();
            out.push(s);
        }

        out
    }

    // Synthesizes a mono musical progression for jukebox fixtures: each frequency
    // in `notes` played sequentially for `note_secs`, then all `chord`
    // frequencies summed for `chord_secs`, repeated `repeats` times. Melody
    // segments use amplitude 0.4 with a 5 ms linear fade in/out to suppress click
    // transients (which would leak broadband energy and pollute "absent note"
    // assertions); chord partials use 0.20 each so a triad peaks at ≤ 0.60.
    pub fn progression(
        notes: &[f32],
        chord: &[f32],
        note_secs: f32,
        chord_secs: f32,
        repeats: u32,
        sample_rate: u32,
    ) -> Vec<f32> {
        let sr = sample_rate as f32;
        let fade_samples = ((sr * 0.005).round() as usize).max(1);
        let seg_samples = (sr * note_secs).round() as usize;
        let chord_samples = (sr * chord_secs).round() as usize;
        let mut out =
            Vec::with_capacity((seg_samples * notes.len() + chord_samples) * repeats as usize);

        for _ in 0..repeats {
            for &freq in notes {
                for i in 0..seg_samples {
                    let t = i as f32 / sr;
                    let mut amp = 0.4_f32;
                    if i < fade_samples {
                        amp *= i as f32 / fade_samples as f32;
                    } else if i >= seg_samples - fade_samples {
                        amp *= (seg_samples - 1 - i) as f32 / fade_samples as f32;
                    }
                    out.push(amp * (2.0 * PI * freq * t).sin());
                }
            }
            for i in 0..chord_samples {
                let t = i as f32 / sr;
                let s: f32 = chord.iter().map(|&f| 0.20 * (2.0 * PI * f * t).sin()).sum();
                out.push(s.clamp(-1.0, 1.0));
            }
        }

        out
    }

    // Linear-frequency sweep (chirp) from `f0` to `f1` over `secs` seconds at
    // `sample_rate`, amplitude ~0.5 so it sits well above any silence floor.
    // The instantaneous phase integrates a frequency that rises linearly with
    // time: phi(t) = 2*pi * (f0*t + 0.5*k*t^2) where k = (f1 - f0) / secs.
    pub fn chirp(sample_rate: u32, secs: f32, f0: f32, f1: f32) -> Vec<f32> {
        let n = (sample_rate as f32 * secs).round() as usize;
        let k = if secs > 0.0 { (f1 - f0) / secs } else { 0.0 };
        let mut out = Vec::with_capacity(n);
        for i in 0..n {
            let t = i as f32 / sample_rate as f32;
            let phase = 2.0 * PI * (f0 * t + 0.5 * k * t * t);
            out.push(0.5 * phase.sin());
        }
        out
    }

    // Root-mean-square amplitude of `x`. Returns 0.0 for an empty slice.
    pub fn rms(x: &[f32]) -> f32 {
        if x.is_empty() {
            return 0.0;
        }
        let sum_sq: f32 = x.iter().map(|s| s * s).sum();
        (sum_sq / x.len() as f32).sqrt()
    }

    // Downmix interleaved stereo (L,R,L,R,...) to mono by averaging each L/R
    // pair. A trailing odd sample (no pair) is passed through unchanged.
    pub fn to_mono(interleaved_stereo: &[f32]) -> Vec<f32> {
        let mut out = Vec::with_capacity(interleaved_stereo.len().div_ceil(2));
        let mut chunks = interleaved_stereo.chunks_exact(2);
        for frame in &mut chunks {
            out.push((frame[0] + frame[1]) * 0.5);
        }
        let rem = chunks.remainder();
        if let Some(&last) = rem.first() {
            out.push(last);
        }
        out
    }

    // Normalized cross-correlation peak in 0..=1 between `reference` and
    // `captured`. Slides a window of `reference.len()` over `captured` and
    // returns the maximum normalized dot product. Handles arbitrary pipeline
    // latency without an O(n²) scan by using a stride pass first to locate
    // the approximate peak, then a fine pass around it.
    //
    // Returns 1.0 for identical signals at the best-match offset; ~0.0 for
    // uncorrelated. Lower than 1.0 after Opus encode/decode, spatial mixing,
    // and timing skew.
    pub fn xcorr_peak(reference: &[f32], captured: &[f32]) -> f32 {
        if reference.is_empty() || captured.is_empty() {
            return 0.0;
        }

        let ref_norm = Self::l2_norm(reference);
        let cap_norm = Self::l2_norm(captured);
        if ref_norm == 0.0 || cap_norm == 0.0 {
            return 0.0;
        }

        let ref_len = reference.len();
        let cap_len = captured.len();

        if cap_len < ref_len {
            // captured is shorter: only scan a small lag range
            let max_lag = cap_len.min(4096);
            let denom = ref_norm * cap_norm;
            let mut best = 0.0_f32;
            for lag in 0..max_lag {
                let overlap = (ref_len - lag).min(cap_len);
                if overlap == 0 {
                    break;
                }
                let mut dot = 0.0_f32;
                for i in 0..overlap {
                    dot += reference[i + lag] * captured[i];
                }
                let corr = (dot / denom).abs();
                if corr > best {
                    best = corr;
                }
            }
            return best.min(1.0);
        }

        // captured >= reference: slide reference over captured.
        // The maximum number of positions is cap_len - ref_len + 1.
        // Use a two-pass approach: coarse scan with stride=ref_len/16, then
        // a fine scan in the neighborhood of the best coarse position.
        let positions = cap_len - ref_len + 1;
        let stride = (ref_len / 16).max(1);

        // Compute the L2 norm of the reference once; for each window position
        // compute the window norm so the denominator is per-window.
        let ref_norm_sq = ref_norm * ref_norm;

        let dot_at = |offset: usize| -> f32 {
            let mut dot = 0.0_f32;
            for i in 0..ref_len {
                dot += reference[i] * captured[offset + i];
            }
            dot
        };

        let window_norm_sq = |offset: usize| -> f32 {
            captured[offset..offset + ref_len]
                .iter()
                .map(|s| s * s)
                .sum::<f32>()
        };

        let normalized_corr = |offset: usize| -> f32 {
            let dot = dot_at(offset);
            let wn = window_norm_sq(offset);
            if wn == 0.0 {
                return 0.0;
            }
            (dot / (ref_norm_sq * wn).sqrt()).abs()
        };

        // Coarse pass
        let mut best_pos = 0usize;
        let mut best_corr = 0.0_f32;
        let mut pos = 0;
        while pos < positions {
            let corr = normalized_corr(pos);
            if corr > best_corr {
                best_corr = corr;
                best_pos = pos;
            }
            pos += stride;
        }

        // Fine pass: ±stride around the best coarse position
        let fine_start = best_pos.saturating_sub(stride);
        let fine_end = (best_pos + stride + 1).min(positions);
        for pos in fine_start..fine_end {
            let corr = normalized_corr(pos);
            if corr > best_corr {
                best_corr = corr;
            }
        }

        best_corr.min(1.0)
    }

    // Fraction of total signal energy that falls within [lo, hi] Hz, in 0..=1.
    // Energy is summed over a coarse bin grid evaluated with Goertzel: each bin
    // center contributes its magnitude-squared, and the in-band fraction is the
    // in-band sum over the total sum. Coarse resolution is sufficient for the
    // band-occupancy assertions tests make.
    pub fn band_energy_ratio(x: &[f32], sample_rate: u32, lo: f32, hi: f32) -> f32 {
        if x.is_empty() || sample_rate == 0 || hi <= lo {
            return 0.0;
        }

        // Coarse bin grid up to Nyquist. ~64 bins gives adequate separation for
        // the wide bands tests check without the cost of a full DFT.
        let nyquist = sample_rate as f32 / 2.0;
        let bins = 64usize;
        let step = nyquist / bins as f32;

        let mut total = 0.0_f32;
        let mut in_band = 0.0_f32;
        for b in 0..bins {
            let freq = (b as f32 + 0.5) * step;
            let power = Self::goertzel_power(x, sample_rate, freq);
            total += power;
            if freq >= lo && freq <= hi {
                in_band += power;
            }
        }

        if total == 0.0 {
            return 0.0;
        }
        (in_band / total).clamp(0.0, 1.0)
    }

    // Fraction of total signal energy concentrated at a single tone `freq` Hz,
    // using Goertzel directly at that frequency. The denominator is the total
    // signal power (sum of squares / n). Use this — not `band_energy_ratio` —
    // when the target frequencies are closer together than the coarse bin grid
    // used by `band_energy_ratio` (375 Hz at 48 kHz / 64 bins). Returns 0.0 for
    // an empty or silent slice.
    pub fn tone_energy_fraction(x: &[f32], sample_rate: u32, freq: f32) -> f32 {
        if x.is_empty() || sample_rate == 0 {
            return 0.0;
        }
        let total_power: f32 = x.iter().map(|s| s * s).sum::<f32>() / x.len() as f32;
        if total_power == 0.0 {
            return 0.0;
        }
        let note_power = Self::goertzel_power(x, sample_rate, freq);
        (note_power / total_power).clamp(0.0, 1.0)
    }

    // Euclidean (L2) norm of a slice.
    fn l2_norm(x: &[f32]) -> f32 {
        x.iter().map(|s| s * s).sum::<f32>().sqrt()
    }

    // Goertzel single-bin power estimate for `target_freq` over `x`. Returns the
    // squared magnitude of the DFT coefficient at that frequency.
    fn goertzel_power(x: &[f32], sample_rate: u32, target_freq: f32) -> f32 {
        let n = x.len();
        let omega = 2.0 * PI * target_freq / sample_rate as f32;
        let coeff = 2.0 * omega.cos();

        let mut s_prev = 0.0_f32;
        let mut s_prev2 = 0.0_f32;
        for &sample in x {
            let s = sample + coeff * s_prev - s_prev2;
            s_prev2 = s_prev;
            s_prev = s;
        }

        let real = s_prev - s_prev2 * omega.cos();
        let imag = s_prev2 * omega.sin();
        (real * real + imag * imag) / (n as f32)
    }
}

#[cfg(test)]
mod tests {
    use super::Signal;

    // Verify that musical_probe concentrates energy at each of the three C-major
    // triad frequencies (C4, E4, G4). Each note must account for at least 5 % of
    // total signal power when evaluated with Goertzel directly at that frequency.
    // The melody and chord together give each note ample representation.
    // `tone_energy_fraction` is used here (not `band_energy_ratio`) because the
    // triad notes are only ~68–63 Hz apart, far below the coarse band_energy_ratio
    // grid resolution (375 Hz at 48 kHz / 64 bins).
    #[test]
    fn musical_probe_concentrates_energy_at_triad_frequencies() {
        let sr = 48_000u32;
        let probe = Signal::musical_probe(sr);

        for (name, freq) in [("C4", super::NOTE_C4), ("E4", super::NOTE_E4), ("G4", super::NOTE_G4)] {
            let fraction = Signal::tone_energy_fraction(&probe, sr, freq);
            assert!(
                fraction > 0.05,
                "musical_probe should concentrate energy at {name} ({freq:.2} Hz), got fraction={fraction:.4}",
            );
        }
    }

    #[test]
    fn progression_contains_each_note_and_excludes_others() {
        let sr = 48_000;
        let notes = [246.94_f32, 293.66, 369.99];
        // Bm triad (B3, D4, F#4) and a well-separated high A-major triad
        // (A5, C#6, E6). The high register keeps the two progressions' Goertzel
        // bins disjoint so the jukebox cross-bleed scenario can assert one set
        // present and the other absent — the single-bin Goertzel leaks badly
        // between near neighbours (e.g. F#4 vs A4 only 70 Hz apart).
        let bm = [246.94_f32, 293.66, 369.99];
        let ahi = [880.00_f32, 1108.73, 1318.51];
        let bm_pcm = Signal::progression(&bm, &bm, 0.4, 0.6, 1, sr);
        let ahi_pcm = Signal::progression(&ahi, &ahi, 0.4, 0.6, 1, sr);
        assert!((bm_pcm.len() as f32 / sr as f32 - 1.8).abs() < 0.05);
        // Each progression's own notes dominate; the other set's notes barely leak.
        for f in bm {
            assert!(
                Signal::tone_energy_fraction(&bm_pcm, sr, f) > 0.02,
                "Bm note {f} Hz missing from Bm progression"
            );
            assert!(
                Signal::tone_energy_fraction(&ahi_pcm, sr, f) < 0.01,
                "Bm note {f} Hz must barely leak into the A-high progression"
            );
        }
        for f in ahi {
            assert!(
                Signal::tone_energy_fraction(&ahi_pcm, sr, f) > 0.02,
                "A-high note {f} Hz missing from A-high progression"
            );
            assert!(
                Signal::tone_energy_fraction(&bm_pcm, sr, f) < 0.01,
                "A-high note {f} Hz must barely leak into the Bm progression"
            );
        }
    }

    #[test]
    fn identical_signals_correlate_near_one() {
        let tone = Signal::chirp(48_000, 0.1, 500.0, 4_000.0);
        let peak = Signal::xcorr_peak(&tone, &tone);
        assert!(peak > 0.95, "identical signals should correlate, got {peak}");
    }

    #[test]
    fn silence_versus_tone_barely_correlates() {
        let tone = Signal::chirp(48_000, 0.1, 500.0, 4_000.0);
        let silence = vec![0.0_f32; tone.len()];
        let peak = Signal::xcorr_peak(&tone, &silence);
        assert!(peak < 0.2, "silence vs tone should not correlate, got {peak}");
    }

    #[test]
    fn pure_tone_energy_lands_in_its_band() {
        let sr = 48_000u32;
        let n = (sr as f32 * 0.1) as usize;
        let mut tone = Vec::with_capacity(n);
        for i in 0..n {
            let t = i as f32 / sr as f32;
            tone.push(0.5 * (2.0 * std::f32::consts::PI * 1_000.0 * t).sin());
        }
        let ratio = Signal::band_energy_ratio(&tone, sr, 800.0, 1_200.0);
        assert!(ratio > 0.8, "1kHz tone should concentrate in [800,1200], got {ratio}");
    }

    #[test]
    fn rms_of_silence_is_near_zero() {
        let silence = vec![0.0_f32; 4_800];
        let rms = Signal::rms(&silence);
        assert!(rms.abs() < 1e-6, "silence rms should be ~0, got {rms}");
    }

    #[test]
    fn to_mono_averages_each_stereo_frame() {
        // L,R pairs: (1,-1)->0, (0.5,0.5)->0.5, (1,0)->0.5
        let stereo = [1.0, -1.0, 0.5, 0.5, 1.0, 0.0];
        let mono = Signal::to_mono(&stereo);
        assert_eq!(mono, vec![0.0, 0.5, 0.5]);
    }
}
