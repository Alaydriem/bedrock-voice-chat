use bvc_client_lib::audio::recording::renderer::{SettingsProvenance, SpatialRenderSettings};
use bvc_client_lib::audio::spatial::SpatialResolver;
use common::players::MinecraftPlayer;
use common::structs::SpatialAudioConfig;
use common::structs::audio::PlayerGainSettings;
use common::structs::recording::{
    InputRecordingHeader, OutputRecordingHeader, PlayerMetadata, RecordingHeader,
};
use common::{Coordinate, Dimension, Orientation, PlayerEnum};

fn player(name: &str, x: f32) -> PlayerEnum {
    PlayerEnum::Minecraft(MinecraftPlayer {
        name: name.to_string(),
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

fn metadata(player_data: Option<PlayerEnum>, gain: Option<f32>) -> PlayerMetadata {
    PlayerMetadata {
        player_data,
        spatial: Some(true),
        gain_settings: gain.map(|gain| PlayerGainSettings {
            gain,
            muted: false,
            last_seen: None,
        }),
    }
}

fn muted(player_data: Option<PlayerEnum>) -> PlayerMetadata {
    PlayerMetadata {
        player_data,
        spatial: Some(true),
        gain_settings: Some(PlayerGainSettings {
            gain: 1.0,
            muted: true,
            last_seen: None,
        }),
    }
}

fn output(emitter: PlayerMetadata, listener: PlayerMetadata, is_spatial: bool) -> RecordingHeader {
    RecordingHeader::Output(OutputRecordingHeader {
        sample_rate: 48000,
        channels: 1,
        relative_timestamp_ms: 0,
        emitter_metadata: emitter,
        listener_metadata: listener,
        is_spatial,
    })
}

fn resolver() -> SpatialResolver {
    SpatialResolver::new(SpatialRenderSettings::new(
        SpatialAudioConfig::default(),
        1.0,
        SettingsProvenance::Defaults,
    ))
}

// Your own voice has no listener behind it and was never positioned on the way in.
#[test]
fn an_input_header_cannot_be_positioned() {
    let header = RecordingHeader::Input(InputRecordingHeader {
        sample_rate: 48000,
        channels: 1,
        relative_timestamp_ms: Some(0),
        emitter_metadata: metadata(Some(player("Alaydriem", 0.0)), None),
    });

    assert!(resolver().gains(&header).is_none());
}

// A frame that went to the flat sink was heard flat, and rendering it positioned would describe
// something that did not happen.
#[test]
fn a_frame_that_was_not_spatial_cannot_be_positioned() {
    let header = output(
        metadata(Some(player("Petra", 30.0)), None),
        metadata(Some(player("Alaydriem", 0.0)), None),
        false,
    );

    assert!(resolver().gains(&header).is_none());
}

// The local player is often absent from the cache for the first frames of a session.
#[test]
fn a_missing_listener_cannot_be_positioned() {
    let header = output(
        metadata(Some(player("Petra", 30.0)), None),
        metadata(None, None),
        true,
    );

    assert!(resolver().gains(&header).is_none());
}

#[test]
fn a_missing_emitter_cannot_be_positioned() {
    let header = output(
        metadata(None, None),
        metadata(Some(player("Alaydriem", 0.0)), None),
        true,
    );

    assert!(resolver().gains(&header).is_none());
}

#[test]
fn a_complete_header_positions_the_emitter_on_the_correct_side() {
    let header = output(
        metadata(Some(player("Petra", 30.0)), None),
        metadata(Some(player("Alaydriem", 0.0)), None),
        true,
    );

    let gains = resolver().gains(&header).expect("a positioned frame");

    assert!(gains.left > gains.right);
}

// The gain the listener had set for that speaker is part of what they heard.
#[test]
fn a_recorded_gain_below_unity_lowers_the_volume() {
    let quiet = output(
        metadata(Some(player("Petra", 10.0)), Some(0.5)),
        metadata(Some(player("Alaydriem", 0.0)), None),
        true,
    );
    let unity = output(
        metadata(Some(player("Petra", 10.0)), Some(1.0)),
        metadata(Some(player("Alaydriem", 0.0)), None),
        true,
    );

    let quiet = resolver().gains(&quiet).expect("a positioned frame");
    let unity = resolver().gains(&unity).expect("a positioned frame");

    assert!(quiet.volume < unity.volume);
}

// A header with no gain recorded is not a reason to silence a track.
#[test]
fn an_absent_gain_is_treated_as_unity() {
    let absent = output(
        metadata(Some(player("Petra", 10.0)), None),
        metadata(Some(player("Alaydriem", 0.0)), None),
        true,
    );
    let unity = output(
        metadata(Some(player("Petra", 10.0)), Some(1.0)),
        metadata(Some(player("Alaydriem", 0.0)), None),
        true,
    );

    let absent = resolver().gains(&absent).expect("a positioned frame");
    let unity = resolver().gains(&unity).expect("a positioned frame");

    assert!((absent.volume - unity.volume).abs() < 1e-6);
}

// A muted emitter's frames never reach the recorder, so this cannot arise from a real session.
// It is implemented because the field is there, and pinned so it cannot quietly invert.
#[test]
fn a_recorded_mute_silences_the_frame() {
    let header = output(
        muted(Some(player("Petra", 10.0))),
        metadata(Some(player("Alaydriem", 0.0)), None),
        true,
    );

    let gains = resolver().gains(&header).expect("a positioned frame");

    assert_eq!(gains.volume, 0.0);
}

// The listener's panning intensity belongs to the render, not the recording, so the same header
// resolves differently under a different setting.
#[test]
fn the_panning_intensity_narrows_what_the_header_resolves_to() {
    let header = output(
        metadata(Some(player("Petra", 30.0)), None),
        metadata(Some(player("Alaydriem", 0.0)), None),
        true,
    );

    let wide = SpatialResolver::new(SpatialRenderSettings::new(
        SpatialAudioConfig::default(),
        1.0,
        SettingsProvenance::Defaults,
    ));
    let narrow = SpatialResolver::new(SpatialRenderSettings::new(
        SpatialAudioConfig::default(),
        0.25,
        SettingsProvenance::Defaults,
    ));

    let wide = wide.gains(&header).expect("a positioned frame");
    let narrow = narrow.gains(&header).expect("a positioned frame");

    assert!(narrow.left < wide.left);
    assert!(narrow.right > wide.right);
}
