use bvc_client_lib::audio::recording::renderer::{DecodedAudioFrame, TrackMixer};

fn frame(at_ms: u64, samples: &[f32]) -> DecodedAudioFrame {
    DecodedAudioFrame {
        pcm_data: samples.to_vec(),
        sample_rate: 48_000,
        channels: 1,
        relative_timestamp_ms: at_ms,
    }
}

#[test]
fn one_source_comes_back_unchanged() {
    let mixed = TrackMixer::mix(&[vec![frame(0, &[0.5, -0.5])]], 48_000, 1);

    assert_eq!(mixed, vec![0.5, -0.5]);
}

#[test]
fn sources_that_overlap_are_summed() {
    let mixed = TrackMixer::mix(
        &[vec![frame(0, &[0.25, 0.25])], vec![frame(0, &[0.25, 0.25])]],
        48_000,
        1,
    );

    assert_eq!(mixed, vec![0.5, 0.5]);
}

// The second source starts a millisecond in, so the timeline has to be long enough for
// both and the first millisecond has to hold only the first source.
#[test]
fn a_later_source_lands_at_its_own_offset() {
    let mixed = TrackMixer::mix(&[vec![frame(0, &[1.0])], vec![frame(1, &[1.0])]], 1_000, 1);

    assert_eq!(mixed.len(), 2);
    assert_eq!(mixed[0], 1.0);
    assert_eq!(mixed[1], 1.0);
}

#[test]
fn a_gap_between_sources_is_silence_and_not_a_shortened_track() {
    let mixed = TrackMixer::mix(&[vec![frame(0, &[1.0]), frame(3, &[1.0])]], 1_000, 1);

    assert_eq!(mixed, vec![1.0, 0.0, 0.0, 1.0]);
}

#[test]
fn a_sum_past_full_scale_is_clamped_rather_than_wrapped() {
    let loud = TrackMixer::mix(&[vec![frame(0, &[0.8])], vec![frame(0, &[0.8])]], 1_000, 1);
    let quiet = TrackMixer::mix(&[vec![frame(0, &[-0.8])], vec![frame(0, &[-0.8])]], 1_000, 1);

    assert_eq!(loud, vec![1.0]);
    assert_eq!(quiet, vec![-1.0]);
}

#[test]
fn no_sources_is_an_empty_track_and_not_a_panic() {
    assert!(TrackMixer::mix(&[], 48_000, 1).is_empty());
}

// A track that starts late says so with a timecode, the way every other track does.
// Carrying the wait as encoded silence would put megabytes of nothing in the file.
#[test]
fn a_late_start_becomes_a_timecode_rather_than_silence() {
    let (samples, lead) =
        TrackMixer::mix_from_first_sound(&[vec![frame(3, &[1.0, 0.5])]], 1_000, 1);

    assert_eq!(lead, 3);
    assert_eq!(samples, vec![1.0, 0.5]);
}

#[test]
fn the_lead_is_the_earliest_source_and_not_the_first_one_listed() {
    let sources = vec![vec![frame(9, &[1.0])], vec![frame(2, &[1.0])]];

    assert_eq!(TrackMixer::lead_ms(&sources), 2);
}

// Trimming must not slide the sources against each other: the gap between them is what
// keeps the mix in time with itself.
#[test]
fn trimming_the_lead_keeps_the_spacing_between_sources() {
    let (samples, lead) = TrackMixer::mix_from_first_sound(
        &[vec![frame(2, &[1.0])], vec![frame(4, &[1.0])]],
        1_000,
        1,
    );

    assert_eq!(lead, 2);
    assert_eq!(samples, vec![1.0, 0.0, 1.0]);
}

#[test]
fn nothing_to_trim_leaves_the_mix_alone() {
    let (samples, lead) = TrackMixer::mix_from_first_sound(&[vec![frame(0, &[0.5])]], 1_000, 1);

    assert_eq!(lead, 0);
    assert_eq!(samples, vec![0.5]);
}
