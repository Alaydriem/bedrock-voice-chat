use bvc_client_lib::audio::recording::renderer::{
    DecodedAudioFrame, SettingsProvenance, SpatialRenderSettings, SpatialSource,
};
use bvc_client_lib::audio::spatial::SpatialResolver;
use common::players::MinecraftPlayer;
use common::structs::SpatialAudioConfig;
use common::structs::recording::{OutputRecordingHeader, PlayerMetadata, RecordingHeader};
use common::{Coordinate, Dimension, Orientation, PlayerEnum};

const RATE: u32 = 48000;
// One 20ms Opus frame at 48kHz.
const FRAME_SAMPLES: usize = 960;

fn player(x: f32) -> PlayerEnum {
    PlayerEnum::Minecraft(MinecraftPlayer {
        name: "Petra".to_string(),
        coordinates: Coordinate { x, y: 0.0, z: 0.0 },
        orientation: Orientation { x: 0.0, y: 0.0 },
        dimension: Dimension::Overworld,
        deafen: false,
        spectator: false,
        world_uuid: None,
        alternative_identity: None,
        player_uuid: None,
        relay_world_uuid: None,
        bridged_voice: false,
    })
}

fn header(emitter_x: Option<f32>, at_ms: u64) -> RecordingHeader {
    RecordingHeader::Output(OutputRecordingHeader {
        sample_rate: RATE,
        channels: 1,
        relative_timestamp_ms: at_ms,
        emitter_metadata: PlayerMetadata {
            player_data: emitter_x.map(player),
            spatial: Some(true),
            gain_settings: None,
        },
        listener_metadata: PlayerMetadata {
            player_data: Some(player(0.0)),
            spatial: Some(true),
            gain_settings: None,
        },
        is_spatial: true,
    })
}

fn frame(at_ms: u64) -> DecodedAudioFrame {
    DecodedAudioFrame {
        pcm_data: vec![0.5f32; FRAME_SAMPLES],
        sample_rate: RATE,
        channels: 1,
        relative_timestamp_ms: at_ms,
    }
}

fn resolver_at(panning_intensity: f32) -> SpatialResolver {
    SpatialResolver::new(SpatialRenderSettings::new(
        SpatialAudioConfig::default(),
        panning_intensity,
        SettingsProvenance::Defaults,
    ))
}

fn resolver() -> SpatialResolver {
    resolver_at(1.0)
}

fn energy(samples: &[f32], offset: usize) -> f32 {
    samples
        .iter()
        .skip(offset)
        .step_by(2)
        .map(|sample| sample * sample)
        .sum()
}

#[test]
fn a_positioned_frame_is_stereo_and_twice_as_long() {
    let positioned = SpatialSource::position(vec![(header(Some(30.0), 0), frame(0))], &resolver());

    assert_eq!(positioned.len(), 1);
    assert_eq!(positioned[0].channels, 2);
    assert_eq!(positioned[0].pcm_data.len(), FRAME_SAMPLES * 2);
    assert_eq!(positioned[0].relative_timestamp_ms, 0);
    assert_eq!(positioned[0].sample_rate, RATE);
}

// Minecraft yaw 0 faces +Z, so an emitter at +X is on the listener's left.
#[test]
fn an_emitter_on_the_left_puts_more_energy_in_the_left_channel() {
    // Far enough out that panning is not suppressed, and two frames for the ramp to converge.
    let frames = vec![
        (header(Some(30.0), 0), frame(0)),
        (header(Some(30.0), 20), frame(20)),
    ];

    let positioned = SpatialSource::position(frames, &resolver());
    let last = &positioned[1].pcm_data;

    assert!(
        energy(last, 0) > energy(last, 1),
        "left {} was not greater than right {}",
        energy(last, 0),
        energy(last, 1)
    );
}

#[test]
fn an_emitter_beyond_the_falloff_distance_renders_silent() {
    let beyond = SpatialAudioConfig::default().falloff_distance + 10.0;
    // Long enough for the ramp to reach zero from its centred, unity start.
    let frames: Vec<_> = (0..40)
        .map(|index| {
            let at = index * 20;
            (header(Some(beyond), at), frame(at))
        })
        .collect();

    let positioned = SpatialSource::position(frames, &resolver());
    let last = positioned.last().expect("frames were returned");

    assert!(
        last.pcm_data.iter().all(|sample| sample.abs() < 1e-3),
        "expected silence, found a sample above the threshold"
    );
}

// A voice with no listener behind it starts centred at unity rather than hard-panned, which is
// the same state the playback sink seeds a fresh spatial voice with.
#[test]
fn a_first_frame_that_cannot_be_positioned_starts_centred() {
    let positioned = SpatialSource::position(vec![(header(None, 0), frame(0))], &resolver());
    let samples = &positioned[0].pcm_data;

    assert!((energy(samples, 0) - energy(samples, 1)).abs() < 1e-3);
    assert!(energy(samples, 0) > 0.0);
}

// A gap in speech must not reset the position. Playback's sink keeps pulling samples through a
// pause, so the voice resumes where it was rather than jumping back to centre.
//
// A half-intensity field keeps both channels well away from zero, so the ratio either side of the
// gap is a stable number to compare.
#[test]
fn the_target_holds_across_a_gap_rather_than_resetting() {
    let mut frames: Vec<_> = (0..40)
        .map(|index| {
            let at = index * 20;
            (header(Some(30.0), at), frame(at))
        })
        .collect();
    // Two seconds of nothing, then one more frame from the same position.
    frames.push((header(Some(30.0), 2_800), frame(2_800)));

    let positioned = SpatialSource::position(frames, &resolver_at(0.5));
    let before_gap = &positioned[39].pcm_data;
    let after_gap = &positioned[40].pcm_data;

    let before_ratio = energy(before_gap, 0) / energy(before_gap, 1);
    let after_ratio = energy(after_gap, 0) / energy(after_gap, 1);

    assert!(
        before_ratio > 1.5 && before_ratio < 10.0,
        "the reference ratio {} is not in a range this test can compare",
        before_ratio
    );
    assert!(
        (before_ratio - after_ratio).abs() / before_ratio < 0.05,
        "the pan changed across the gap: {} then {}",
        before_ratio,
        after_ratio
    );
}

#[test]
fn no_frames_produce_no_output() {
    let positioned = SpatialSource::position(Vec::new(), &resolver());

    assert!(positioned.is_empty());
}

// A stereo input cannot be panned as it stands, so it is folded down first. Voice frames are mono
// in practice; this is here so an unexpected one does not double the output length.
#[test]
fn a_stereo_input_frame_is_folded_to_mono_before_positioning() {
    let stereo = DecodedAudioFrame {
        pcm_data: vec![0.5f32; FRAME_SAMPLES * 2],
        sample_rate: RATE,
        channels: 2,
        relative_timestamp_ms: 0,
    };
    let mut source_header = header(Some(30.0), 0);
    if let RecordingHeader::Output(inner) = &mut source_header {
        inner.channels = 2;
    }

    let positioned = SpatialSource::position(vec![(source_header, stereo)], &resolver());

    assert_eq!(positioned[0].pcm_data.len(), FRAME_SAMPLES * 2);
    assert_eq!(positioned[0].channels, 2);
}

// The ramp is what keeps a position change from clicking. Two adjacent output samples must never
// jump by more than the ramp can move in one step.
#[test]
fn a_position_change_ramps_rather_than_stepping() {
    let frames = vec![
        (header(Some(-30.0), 0), frame(0)),
        (header(Some(30.0), 20), frame(20)),
    ];

    let positioned = SpatialSource::position(frames, &resolver());
    let second = &positioned[1].pcm_data;

    // Compare like with like: left against left, so the stereo interleave is not read as a jump.
    let biggest_step = second
        .iter()
        .step_by(2)
        .zip(second.iter().step_by(2).skip(1))
        .map(|(a, b)| (b - a).abs())
        .fold(0.0f32, f32::max);

    assert!(
        biggest_step < 0.01,
        "a single-sample step of {} would be audible as a click",
        biggest_step
    );
}
