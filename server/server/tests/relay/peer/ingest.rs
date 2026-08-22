use std::collections::HashMap;
use std::sync::Arc;

use bvc_relay::node::PeerTicket;
use bvc_server_lib::config::PeerConfig;
use bvc_server_lib::relay::{GrantTable, IngestRejection, LocalClients, PeerIngest};
use common::game_data::Dimension;
use common::structs::packet::PacketType;
use common::structs::relay::wire::datagram::VoiceFrame;
use common::traits::player_data::PlayerData;
use common::{Coordinate, MinecraftPlayer, Orientation, PlayerEnum};
use iroh::{EndpointAddr, PublicKey, SecretKey};

// Nobody is connected unless a test says so.
struct NoLocals;

impl LocalClients for NoLocals {
    fn has_live_client(&self, _identity: &str) -> bool {
        false
    }
}

struct OneLocal(&'static str);

impl LocalClients for OneLocal {
    fn has_live_client(&self, identity: &str) -> bool {
        identity == self.0
    }
}

fn speaker(name: &str, world: Option<&str>) -> PlayerEnum {
    PlayerEnum::Minecraft(MinecraftPlayer {
        name: name.to_string(),
        coordinates: Coordinate {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        orientation: Orientation { x: 0.0, y: 0.0 },
        dimension: Dimension::Overworld,
        deafen: false,
        spectator: false,
        world_uuid: None,
        alternative_identity: None,
        player_uuid: None,
        relay_world_uuid: world.map(String::from),
        bridged_voice: false,
    })
}

fn frame(name: &str, world: Option<&str>) -> VoiceFrame {
    VoiceFrame {
        speaker: speaker(name, world),
        sample_rate: 48000,
        opus: vec![0xAA],
        timestamp_ms: 0,
        spatial: true,
        jukebox: None,
    }
}

fn grants_for(node: PublicKey, worlds: &[&str]) -> Arc<GrantTable> {
    let mut map = HashMap::new();
    map.insert(
        "peer".to_string(),
        PeerConfig {
            peerlink: PeerTicket::mint(&EndpointAddr::new(node)).expect("mint"),
            worlds: worlds.iter().map(|w| w.to_string()).collect(),
            capabilities: vec!["carry_speakers".to_string()],
        },
    );
    Arc::new(GrantTable::from_config(&map).expect("valid config"))
}

#[test]
fn a_granted_world_is_admitted_and_the_packet_is_minted_here() {
    let node = SecretKey::generate().public();
    let ingest = PeerIngest::new(grants_for(node, &["W1"]), Arc::new(NoLocals));

    let (packet, speaker) = ingest
        .admit(&node, frame("Alice", Some("W1")))
        .expect("admitted");

    assert_eq!(packet.packet_type, PacketType::AudioFrame);
    assert!(
        packet.sender.is_some(),
        "the sender must be minted here, because the wire carries none"
    );
    // The speaker leaves separately because the caller publishes it where routing can find it.
    // A receiving server has no position feed covering another server's players, so if this
    // stopped travelling, relayed audio would arrive with nowhere to place it.
    assert_eq!(
        speaker.get_name(),
        "Alice",
        "the speaker the wire named comes back out"
    );
    assert_eq!(
        packet.sender_key().as_deref(),
        Some("minecraft:Alice"),
        "under the key it will be published as"
    );
}

#[test]
fn a_world_outside_the_grant_is_refused() {
    let node = SecretKey::generate().public();
    let ingest = PeerIngest::new(grants_for(node, &["W1"]), Arc::new(NoLocals));

    assert!(matches!(
        ingest.admit(&node, frame("Alice", Some("W2"))),
        Err(IngestRejection::NotGranted { .. })
    ));
}

#[test]
fn an_undeclared_node_is_refused() {
    let declared = SecretKey::generate().public();
    let stranger = SecretKey::generate().public();
    let ingest = PeerIngest::new(grants_for(declared, &["W1"]), Arc::new(NoLocals));

    assert!(matches!(
        ingest.admit(&stranger, frame("Alice", Some("W1"))),
        Err(IngestRejection::NotGranted { .. })
    ));
}

// A speaker with no relay world cannot be scoped to a grant, so there is no
// question to which "yes" is a safe answer.
#[test]
fn a_speaker_without_a_relay_world_is_refused() {
    let node = SecretKey::generate().public();
    let ingest = PeerIngest::new(grants_for(node, &["W1"]), Arc::new(NoLocals));

    assert!(matches!(
        ingest.admit(&node, frame("Alice", None)),
        Err(IngestRejection::NoWorld)
    ));
}

// Naming a player this server serves would let a peer overwrite that player's
// cached position and inherit their channel membership, which bypasses the
// proximity gate entirely.
#[test]
fn a_peer_naming_a_locally_connected_player_is_refused() {
    let node = SecretKey::generate().public();
    let ingest = PeerIngest::new(
        grants_for(node, &["W1"]),
        Arc::new(OneLocal("minecraft:Alice")),
    );

    assert!(matches!(
        ingest.admit(&node, frame("Alice", Some("W1"))),
        Err(IngestRejection::ImpersonatesLocalPlayer { .. })
    ));
}

// The same name that is refused when it is ours is admitted when it is not.
#[test]
fn a_peer_naming_a_player_we_do_not_serve_is_admitted() {
    let node = SecretKey::generate().public();
    let ingest = PeerIngest::new(
        grants_for(node, &["W1"]),
        Arc::new(OneLocal("minecraft:Someone")),
    );

    assert!(ingest.admit(&node, frame("Alice", Some("W1"))).is_ok());
}
