use bvc_relay_sdk::SdkFrame;
use common::game_data::Dimension;
use common::structs::relay::wire::datagram::VoiceFrame;
use common::{Coordinate, MinecraftPlayer, Orientation, PlayerEnum};

fn wire(world: Option<&str>) -> VoiceFrame {
    VoiceFrame {
        speaker: PlayerEnum::Minecraft(MinecraftPlayer {
            name: "Alice".to_string(),
            coordinates: Coordinate {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            },
            orientation: Orientation { x: 0.0, y: 0.0 },
            dimension: Dimension::Overworld,
            deafen: false,
            spectator: false,
            world_uuid: None,
            alternative_identity: None,
            player_uuid: None,
            relay_world_uuid: world.map(str::to_string),
        }),
        sample_rate: 48000,
        opus: vec![9, 9],
        timestamp_ms: 42,
        spatial: true,
        jukebox: None,
    }
}

// The bridge maps a speaker to an SVC player by name and places them by
// coordinate, so both have to survive the flattening.
#[test]
fn a_wire_frame_flattens_to_the_ffi_shape() {
    let frame = SdkFrame::from(wire(Some("W1")));

    assert_eq!(frame.speaker, "Alice");
    assert_eq!(frame.world, Some("W1".to_string()));
    assert_eq!(frame.x, 1.0);
    assert_eq!(frame.y, 2.0);
    assert_eq!(frame.z, 3.0);
    assert_eq!(frame.opus, vec![9, 9]);
    assert_eq!(frame.sample_rate, 48000);
    assert_eq!(frame.timestamp_ms, 42);
    assert!(frame.spatial);
}

// A frame the bridge sends must arrive as the same wire type the server admits,
// so the round trip has to preserve what the ingest boundary checks: the speaker
// and the relay world.
#[test]
fn an_ffi_frame_round_trips_through_the_wire_shape() {
    let original = SdkFrame::from(wire(Some("W1")));
    let round_tripped = SdkFrame::from(VoiceFrame::from(original.clone()));

    assert_eq!(round_tripped.speaker, original.speaker);
    assert_eq!(round_tripped.world, original.world);
    assert_eq!(round_tripped.opus, original.opus);
    assert_eq!(round_tripped.x, original.x);
}

// A bridge decides what to do with a playback, so it has to be able to tell one
// from speech. Matching the speaker's name prefix would work today and is not a
// contract this API makes.
#[test]
fn a_jukebox_id_crosses_the_ffi_boundary() {
    let mut wire_frame = wire(Some("W1"));
    wire_frame.jukebox = Some("evt-9".to_string());

    let frame = SdkFrame::from(wire_frame);

    assert_eq!(frame.jukebox, Some("evt-9".to_string()));
    assert_eq!(VoiceFrame::from(frame).jukebox, Some("evt-9".to_string()));
}

#[test]
fn speech_carries_no_jukebox_id() {
    assert!(SdkFrame::from(wire(Some("W1"))).jukebox.is_none());
}

// A speaker with no relay world cannot be admitted by any grant, so the absence
// has to survive rather than becoming an empty string that looks like a world.
#[test]
fn a_speaker_with_no_world_keeps_none() {
    let frame = SdkFrame::from(wire(None));

    assert!(frame.world.is_none());
    assert!(VoiceFrame::from(frame).speaker.as_minecraft().is_some());
}

// The dimension gate in `can_communicate_with` is unconditional, so a frame that
// does not carry the speaker's dimension does not merely omit one — it asserts a
// wrong one, and the audio lands in the wrong world in both directions.
#[test]
fn a_frame_carries_the_speakers_dimension_across_both_conversions() {
    let mut wire_frame = wire(Some("W1"));
    if let PlayerEnum::Minecraft(player) = &mut wire_frame.speaker {
        player.dimension = Dimension::TheNether;
    }

    let frame = SdkFrame::from(wire_frame);
    assert_eq!(frame.dimension, "nether");

    let back = VoiceFrame::from(frame);
    assert_eq!(
        back.speaker.as_minecraft().expect("minecraft").dimension,
        Dimension::TheNether
    );
}

// An unrecognised dimension resolves rather than failing: a bridge for a modded
// dimension must still be audible somewhere, and the wire type has no variant for
// "unknown".
#[test]
fn an_unrecognised_dimension_falls_back_rather_than_failing() {
    let mut frame = SdkFrame::from(wire(Some("W1")));
    frame.dimension = "someones_custom_dim".to_string();

    let back = VoiceFrame::from(frame);

    assert_eq!(
        back.speaker.as_minecraft().expect("minecraft").dimension,
        Dimension::Overworld
    );
}
