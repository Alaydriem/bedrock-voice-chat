use std::collections::HashMap;
use std::sync::Arc;

use bvc_relay::node::PeerTicket;
use bvc_server_lib::config::PeerConfig;
use bvc_server_lib::relay::{GrantTable, LocalClients, PeerEgress, PeerIngest};
use common::game_data::Dimension;
use common::structs::packet::{
    AudioFrameMetadata, AudioFramePacket, JukeboxMetadata, PacketSender, PacketType,
    QuicNetworkPacket, QuicNetworkPacketData,
};
use common::traits::player_data::PlayerData;
use common::{Coordinate, MinecraftPlayer, Orientation, PlayerEnum};
use iroh::{EndpointAddr, PublicKey, SecretKey};

struct NoLocals;

impl LocalClients for NoLocals {
    fn has_live_client(&self, _identity: &str) -> bool {
        false
    }
}

fn speaker(name: &str) -> PlayerEnum {
    PlayerEnum::Minecraft(MinecraftPlayer {
        name: name.to_string(),
        coordinates: Coordinate {
            x: 10.0,
            y: 64.0,
            z: -3.0,
        },
        orientation: Orientation { x: 0.0, y: 0.0 },
        dimension: Dimension::TheNether,
        deafen: false,
        spectator: false,
        world_uuid: None,
        alternative_identity: None,
        player_uuid: None,
        relay_world_uuid: Some("W1".to_string()),
        bridged_voice: false,
    })
}

fn packet_of(name: &str, metadata: Vec<AudioFrameMetadata>) -> QuicNetworkPacket {
    let mut audio = AudioFramePacket::new(vec![1, 2, 3], 48000, Some(speaker(name)), Some(true));
    if !metadata.is_empty() {
        audio = audio.with_metadata(metadata);
    }

    QuicNetworkPacket {
        packet_type: PacketType::AudioFrame,
        sender: Some(PacketSender::synthetic(name)),
        data: QuicNetworkPacketData::AudioFrame(audio),
        ..Default::default()
    }
}

fn jukebox_packet(event_id: &str) -> QuicNetworkPacket {
    packet_of(
        "jukebox-abcd1234",
        vec![AudioFrameMetadata::Jukebox(JukeboxMetadata::new(
            Coordinate {
                x: 10.0,
                y: 64.0,
                z: -3.0,
            },
            event_id.to_string(),
            Dimension::TheNether,
        ))],
    )
}

fn grants_for(node: PublicKey) -> Arc<GrantTable> {
    let mut map = HashMap::new();
    map.insert(
        "bridge".to_string(),
        PeerConfig {
            peerlink: PeerTicket::mint(&EndpointAddr::new(node)).expect("mint"),
            worlds: Vec::new(),
            capabilities: vec!["carry_speakers".to_string()],
        },
    );
    Arc::new(GrantTable::from_config(&map).expect("valid config"))
}

// Speech must not be mistaken for a playback, or every remote speaker would land
// on a jukebox sink and inherit the listener's music setting.
#[test]
fn a_speech_frame_carries_no_jukebox_id() {
    let (_, frame) = PeerEgress::frame_from(&packet_of("Alice", Vec::new())).expect("forwardable");

    assert!(frame.jukebox.is_none());
}

#[test]
fn a_jukebox_frame_carries_its_event_id() {
    let (_, frame) = PeerEgress::frame_from(&jukebox_packet("evt-9")).expect("forwardable");

    assert_eq!(frame.jukebox, Some("evt-9".to_string()));
}

// The far side rebuilds the metadata from the speaker rather than receiving a
// copy of it, so the dimension has to be the speaker's real one rather than a
// default that would place a beacon in the wrong world.
#[test]
fn ingest_rebuilds_the_jukebox_metadata_from_the_speaker() {
    let node = SecretKey::generate().public();
    let ingest = PeerIngest::new(grants_for(node), Arc::new(NoLocals));

    let (_, frame) = PeerEgress::frame_from(&jukebox_packet("evt-9")).expect("forwardable");
    let admitted = ingest.admit(&node, frame).expect("admitted");

    let QuicNetworkPacketData::AudioFrame(audio) = admitted.data else {
        panic!("not an audio frame");
    };

    let jukebox = audio
        .metadata
        .iter()
        .find_map(|meta| match meta {
            AudioFrameMetadata::Jukebox(jb) => Some(jb),
        })
        .expect("jukebox metadata rebuilt");

    assert_eq!(jukebox.event_id, "evt-9");
    assert_eq!(jukebox.dimension, Dimension::TheNether);
    assert_eq!(jukebox.position.y, 64.0);
}

#[test]
fn ingest_adds_no_metadata_to_speech() {
    let node = SecretKey::generate().public();
    let ingest = PeerIngest::new(grants_for(node), Arc::new(NoLocals));

    let (_, frame) = PeerEgress::frame_from(&packet_of("Alice", Vec::new())).expect("forwardable");
    let admitted = ingest.admit(&node, frame).expect("admitted");

    let QuicNetworkPacketData::AudioFrame(audio) = admitted.data else {
        panic!("not an audio frame");
    };

    assert!(audio.metadata.is_empty());
}

// The whole round trip, which is what an operator actually cares about: a
// playback on one server arrives on its peer still identifiable as one.
#[test]
fn a_playback_survives_the_peer_boundary() {
    let node = SecretKey::generate().public();
    let ingest = PeerIngest::new(grants_for(node), Arc::new(NoLocals));

    let (world, frame) = PeerEgress::frame_from(&jukebox_packet("evt-42")).expect("forwardable");
    assert_eq!(world, "W1");

    let admitted = ingest.admit(&node, frame).expect("admitted");
    let QuicNetworkPacketData::AudioFrame(audio) = admitted.data else {
        panic!("not an audio frame");
    };

    // The name is what keys the receiving client's jukebox sink, so it has to
    // arrive intact alongside the metadata.
    assert_eq!(
        audio.sender.as_ref().expect("speaker").get_name(),
        "jukebox-abcd1234"
    );
    assert_eq!(audio.metadata.len(), 1);
}
