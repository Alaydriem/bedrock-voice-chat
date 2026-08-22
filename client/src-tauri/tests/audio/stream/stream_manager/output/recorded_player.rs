use bvc_client_lib::audio::stream::stream_manager::output::RecordedPlayer;
use common::Coordinate;
use common::structs::packet::SpeakerPosition;
use common::traits::player_data::PlayerData;

// The recorded header holds a whole player because that is what it has always held, and the
// renderer reads a position and a deafened flag back out of it. Losing the flag records a
// deafened speaker as attenuated, which is a silent wrong answer in an exported recording.
#[test]
fn a_deafened_speaker_is_recorded_as_deafened() {
    let synthesised = RecordedPlayer::synthesise(
        "minecraft:Alice",
        &SpeakerPosition::new(
            Coordinate {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            },
            true,
        ),
    );

    assert!(synthesised.is_deafened());
}

#[test]
fn the_recorded_position_is_the_one_the_frame_carried() {
    let synthesised = RecordedPlayer::synthesise(
        "minecraft:Alice",
        &SpeakerPosition::new(
            Coordinate {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            },
            false,
        ),
    );

    assert_eq!(synthesised.get_position().x, 1.0);
    assert_eq!(synthesised.get_position().z, 3.0);
    assert!(!synthesised.is_deafened());
}

// The renderer displays the bare gamertag, and the canonical identity is what the caller holds.
// Recording the prefixed form would put "minecraft:Alice" on a track label.
#[test]
fn the_recorded_name_is_the_bare_gamertag() {
    let synthesised = RecordedPlayer::synthesise(
        "minecraft:Alice",
        &SpeakerPosition::new(Coordinate::default(), false),
    );

    assert_eq!(synthesised.get_name(), "Alice");
}
