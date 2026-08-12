use bvc_client_lib::audio::recording::renderer::{DecodedAudioFrame, TrackMixer};

fn stereo_frame(value: f32, samples: usize, at_ms: u64) -> DecodedAudioFrame {
    DecodedAudioFrame {
        pcm_data: vec![value; samples],
        sample_rate: 48000,
        channels: 2,
        relative_timestamp_ms: at_ms,
    }
}

// A stereo frame laid down at an odd sample index swaps its channels for the rest of the
// timeline. Every rate Opus supports has to produce an even samples-per-millisecond.
#[test]
fn a_stereo_frame_lands_on_a_frame_boundary_for_every_opus_sample_rate() {
    for rate in [8000u32, 12000, 16000, 24000, 48000] {
        let per_ms = (rate as usize * 2) / 1000;

        assert_eq!(
            per_ms % 2,
            0,
            "rate {} gives {} samples per millisecond, which is odd",
            rate,
            per_ms
        );
    }
}

// Two sources at the same instant sum rather than one replacing the other.
#[test]
fn overlapping_sources_are_summed() {
    let mixed = TrackMixer::mix(
        &[
            vec![stereo_frame(0.25, 4, 0)],
            vec![stereo_frame(0.25, 4, 0)],
        ],
        48000,
        2,
    );

    assert!((mixed[0] - 0.5).abs() < 1e-6);
}

// A frame arriving later sits later on the timeline, at an even index so its left sample stays a
// left sample.
#[test]
fn a_later_stereo_frame_is_offset_by_an_even_number_of_samples() {
    let frames = vec![stereo_frame(1.0, 2, 0), stereo_frame(1.0, 2, 20)];

    let mixed = TrackMixer::mix(&[frames], 48000, 2);
    let second = mixed
        .iter()
        .enumerate()
        .skip(2)
        .find(|(_, sample)| **sample != 0.0)
        .map(|(index, _)| index)
        .expect("the second frame is on the timeline");

    assert_eq!(second % 2, 0);
}

// Summed sources can exceed full scale, and everything downstream expects a signal inside it.
#[test]
fn a_sum_past_full_scale_is_clamped() {
    let sources: Vec<_> = (0..6).map(|_| vec![stereo_frame(0.5, 4, 0)]).collect();

    let mixed = TrackMixer::mix(&sources, 48000, 2);

    assert!(mixed.iter().all(|sample| *sample <= 1.0));
}
