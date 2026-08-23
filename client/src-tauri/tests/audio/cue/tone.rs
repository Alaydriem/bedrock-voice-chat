use bvc_client_lib::audio::Cue;

/// Rates a real endpoint reports, including the unusual ones.
const RATES: [u32; 5] = [22_050, 44_100, 48_000, 96_000, 192_000];

const ALL: [Cue; 4] = [Cue::Mute, Cue::Unmute, Cue::Deafen, Cue::Undeafen];

/// Sign changes in a slice. A pitch measure that does not care about amplitude, which
/// matters because the window used below sits deep in the decay.
fn zero_crossings(samples: &[f32]) -> usize {
    samples
        .windows(2)
        .filter(|pair| (pair[0] < 0.0) != (pair[1] < 0.0))
        .count()
}

/// The 30 ms at the start, where only the first note is sounding.
fn opening(samples: &[f32], rate: u32) -> &[f32] {
    &samples[..(rate as f32 * 0.030) as usize]
}

/// The 30 ms ending 20 ms before the render does. By here every note but the last is more
/// than three decay constants down, so the last note is what the sign changes count.
fn closing(samples: &[f32], rate: u32, duration: f32) -> &[f32] {
    let end = ((duration - 0.020) * rate as f32) as usize;
    let start = end - (rate as f32 * 0.030) as usize;
    &samples[start..end]
}

/// The whole point of the feature: off falls, on rises. A user learns this in one session
/// without being told, and only if the rendered audio actually does it — a note table that
/// reads correctly but renders flat would pass every other test here.
#[test]
fn turning_something_off_falls_and_turning_it_on_rises() {
    for rate in RATES {
        for cue in ALL {
            let samples = cue.samples(rate, 1);
            let first = zero_crossings(opening(&samples, rate));
            let last = zero_crossings(closing(&samples, rate, cue.duration_seconds()));

            match cue {
                Cue::Mute | Cue::Deafen => assert!(
                    first > last,
                    "{cue:?} at {rate} Hz did not fall: opened at {first} crossings, closed at {last}"
                ),
                Cue::Unmute | Cue::Undeafen => assert!(
                    first < last,
                    "{cue:?} at {rate} Hz did not rise: opened at {first} crossings, closed at {last}"
                ),
            }
        }
    }
}

/// Mute and deafen are separate states with separate consequences, and the user cannot see
/// either from inside the game. Two notes against three is what tells them apart by ear.
#[test]
fn deafen_is_longer_than_mute() {
    assert!(Cue::Deafen.duration_seconds() > Cue::Mute.duration_seconds());
    assert!(Cue::Undeafen.duration_seconds() > Cue::Unmute.duration_seconds());
}

/// These fire dozens of times a session, unlike the speaker test which fires once during
/// setup. A cue at speaker-test level is the thing people turn the feature off over.
#[test]
fn stays_well_below_the_speaker_test_level() {
    for cue in ALL {
        let peak = cue
            .samples(48_000, 1)
            .into_iter()
            .fold(0.0f32, |peak, sample| peak.max(sample.abs()));

        assert!(peak > 0.15, "{cue:?} peaked at {peak}, too quiet to notice");
        assert!(peak < 0.30, "{cue:?} peaked at {peak}, louder than intended");
    }
}

/// A cue that clicks is worse than no cue: it reads as a fault in the audio path at the
/// exact moment the user changed something about the audio path.
#[test]
fn starts_and_ends_without_a_click() {
    for rate in RATES {
        for cue in ALL {
            let samples = cue.samples(rate, 1);

            let first = samples[0].abs();
            assert!(first < 0.02, "{cue:?} at {rate} Hz opened at {first}");

            let last = samples[samples.len() - 1].abs();
            assert!(last < 0.03, "{cue:?} at {rate} Hz ended at {last}");
        }
    }
}

#[test]
fn never_leaves_the_sample_range() {
    for rate in RATES {
        for cue in ALL {
            for sample in cue.samples(rate, 2) {
                assert!(
                    sample.is_finite() && (-1.0..=1.0).contains(&sample),
                    "{cue:?} at {rate} Hz produced {sample}"
                );
            }
        }
    }
}

#[test]
fn fills_the_requested_channels_evenly() {
    for channels in [1u16, 2, 4, 6, 8] {
        for cue in ALL {
            let samples = cue.samples(48_000, channels);
            assert_eq!(
                samples.len() % channels as usize,
                0,
                "{cue:?} left a partial frame at {channels} channels"
            );

            for frame in samples.chunks(channels as usize) {
                assert!(
                    frame.iter().all(|s| *s == frame[0]),
                    "{cue:?} channels disagree within a frame"
                );
            }
        }
    }
}
