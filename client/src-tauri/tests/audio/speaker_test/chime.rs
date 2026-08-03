use bvc_client_lib::audio::Chime;

/// The chime is generated at whatever rate the output device reports, so the properties
/// worth guarding are the ones that hold at every rate: it stays inside the sample range,
/// it starts and ends quietly enough not to click, and every channel carries the signal.
///
/// A device test that clicks, or that only comes out of one side, sends the user looking for
/// a fault in hardware that is working.

/// Rates a real endpoint reports, including the unusual ones.
const RATES: [u32; 5] = [22_050, 44_100, 48_000, 96_000, 192_000];

#[test]
fn fills_the_requested_channels_evenly() {
    for channels in [1u16, 2, 4, 6, 8] {
        let samples = Chime::samples(48_000, channels);
        assert_eq!(
            samples.len() % channels as usize,
            0,
            "{channels} channels left a partial frame"
        );

        // Every channel in a frame is the same value: a chime that favoured one side would
        // read as a broken speaker on the screen whose job is to find broken speakers.
        for frame in samples.chunks(channels as usize) {
            assert!(
                frame.iter().all(|s| *s == frame[0]),
                "channels disagree within a frame"
            );
        }
    }
}

#[test]
fn lasts_about_as_long_as_it_claims() {
    for rate in RATES {
        let frames = Chime::samples(rate, 1).len();
        let seconds = frames as f32 / rate as f32;
        assert!(
            (seconds - Chime::DURATION_SECONDS).abs() < 0.01,
            "{rate} Hz produced {seconds}s"
        );
    }
}

#[test]
fn never_leaves_the_sample_range() {
    for rate in RATES {
        for sample in Chime::samples(rate, 2) {
            assert!(
                sample.is_finite() && (-1.0..=1.0).contains(&sample),
                "{rate} Hz produced {sample}"
            );
        }
    }
}

/// The attack ramp and the decay exist to avoid a discontinuity at either end. A step from
/// or to zero is a click, and on a speaker test a click is indistinguishable from the
/// hardware fault the user is trying to rule out.
#[test]
fn starts_and_ends_without_a_click() {
    for rate in RATES {
        let samples = Chime::samples(rate, 1);

        let first = samples[0].abs();
        assert!(first < 0.02, "{rate} Hz opened at {first}");

        let last = samples[samples.len() - 1].abs();
        assert!(last < 0.05, "{rate} Hz ended at {last}");
    }
}

/// Loud enough to hear, with headroom left. Normalisation is measured from the generated
/// peak, so this is what catches a partial being added without the level being rechecked.
#[test]
fn reaches_a_usable_level_with_headroom() {
    let peak = Chime::samples(48_000, 1)
        .into_iter()
        .fold(0.0f32, |peak, sample| peak.max(sample.abs()));

    assert!(peak > 0.3, "too quiet to serve as a test: {peak}");
    assert!(peak < 0.8, "no headroom left: {peak}");
}

/// Energy at one frequency, by Goertzel. Cheaper than a transform and enough to answer
/// whether a given note is sounding in a given window.
fn magnitude_at(samples: &[f32], rate: u32, frequency: f32) -> f32 {
    let w = std::f32::consts::TAU * frequency / rate as f32;
    let coefficient = 2.0 * w.cos();
    let (mut s1, mut s2) = (0.0f32, 0.0f32);

    for &sample in samples {
        let s0 = sample + coefficient * s1 - s2;
        s2 = s1;
        s1 = s0;
    }

    (s1 * s1 + s2 * s2 - coefficient * s1 * s2)
        .max(0.0)
        .sqrt()
        / samples.len() as f32
}

/// Both notes have to sound. The second is what makes the chime recognisable as deliberate
/// rather than as a system error sound, and a decay tuned too short would swallow it.
///
/// Measured spectrally rather than by amplitude: the two notes overlap and the signal is
/// peak-normalised, so windows either side of the onset both sit near full scale and tell
/// you nothing. Asking whether the second note's own frequency is present does.
#[test]
fn both_notes_sound() {
    let rate = 48_000;
    let samples = Chime::samples(rate, 1);
    let second_note = 1174.66;

    let window = |from: f32, len: f32| {
        let start = (rate as f32 * from) as usize;
        let end = start + (rate as f32 * len) as usize;
        &samples[start..end.min(samples.len())]
    };

    let before = magnitude_at(window(0.05, 0.06), rate, second_note);
    let after = magnitude_at(window(0.18, 0.06), rate, second_note);

    assert!(
        after > before * 4.0,
        "the second note's frequency is not arriving: {before} before, {after} after"
    );
}

/// And the first note has to be the one that opens it. A chime whose notes swapped order
/// would still pass the test above.
#[test]
fn the_first_note_opens_it() {
    let rate = 48_000;
    let samples = Chime::samples(rate, 1);

    let opening = &samples[(rate as f32 * 0.02) as usize..(rate as f32 * 0.08) as usize];
    let first = magnitude_at(opening, rate, 880.0);
    let second = magnitude_at(opening, rate, 1174.66);

    assert!(
        first > second * 4.0,
        "the opening is not the first note: {first} at 880 Hz, {second} at 1174 Hz"
    );
}
