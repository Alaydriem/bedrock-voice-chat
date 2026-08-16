use bvc_server_lib::relay::PeerEgress;
use common::game_data::Dimension;
use common::structs::packet::{
    AudioFramePacket, HealthCheckPacket, PacketType, QuicNetworkPacket, QuicNetworkPacketData,
};
use common::{Coordinate, MinecraftPlayer, Orientation, PlayerEnum};

fn speaker(world: Option<&str>) -> PlayerEnum {
    PlayerEnum::Minecraft(MinecraftPlayer {
        name: "Alice".to_string(),
        coordinates: Coordinate {
            x: 5.0,
            y: 6.0,
            z: 7.0,
        },
        orientation: Orientation { x: 0.0, y: 0.0 },
        dimension: Dimension::Overworld,
        deafen: false,
        spectator: false,
        world_uuid: None,
        alternative_identity: None,
        player_uuid: None,
        relay_world_uuid: world.map(String::from),
    })
}

fn audio(sender: Option<PlayerEnum>) -> QuicNetworkPacket {
    QuicNetworkPacket {
        packet_type: PacketType::AudioFrame,
        data: QuicNetworkPacketData::AudioFrame(AudioFramePacket::new(
            vec![1, 2, 3],
            48000,
            sender,
            Some(true),
        )),
        ..Default::default()
    }
}

#[test]
fn an_audio_packet_with_a_relay_world_becomes_a_frame_for_that_world() {
    let (world, frame) =
        PeerEgress::frame_from(&audio(Some(speaker(Some("W1"))))).expect("forwardable");

    assert_eq!(world, "W1");
    assert_eq!(frame.opus, vec![1, 2, 3]);
    assert_eq!(frame.sample_rate, 48000);
    assert!(frame.spatial);
}

// The speaker's coordinates must survive, because the far side has no position
// feed covering our players.
#[test]
fn the_speakers_coordinates_survive_the_conversion() {
    let (_, frame) =
        PeerEgress::frame_from(&audio(Some(speaker(Some("W1"))))).expect("forwardable");
    let mc = frame.speaker.as_minecraft().expect("minecraft speaker");

    assert_eq!(mc.coordinates.x, 5.0);
    assert_eq!(mc.coordinates.y, 6.0);
    assert_eq!(mc.coordinates.z, 7.0);
}

#[test]
fn a_packet_with_no_relay_world_is_not_forwardable() {
    assert!(PeerEgress::frame_from(&audio(Some(speaker(None)))).is_none());
}

#[test]
fn a_packet_with_no_sender_is_not_forwardable() {
    assert!(PeerEgress::frame_from(&audio(None)).is_none());
}

#[test]
fn a_non_audio_packet_is_not_forwardable() {
    let packet = QuicNetworkPacket {
        packet_type: PacketType::HealthCheck,
        data: QuicNetworkPacketData::HealthCheck(HealthCheckPacket),
        ..Default::default()
    };

    assert!(PeerEgress::frame_from(&packet).is_none());
}
