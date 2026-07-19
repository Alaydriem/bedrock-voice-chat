use std::sync::Arc;

use bvc_client_lib::NetworkPacket;
use bvc_client_lib::bedrock::JukeboxBeaconCache;
use bvc_client_lib::bedrock::BedrockEventEmitter;
use bvc_client_lib::bedrock::proxy::session::handlers::PlaySoundHandler;
use bvc_client_lib::bedrock::proxy::session::{BedrockPacketHandler, BedrockSessionState};
use common::bedrock_protocol::protocol::packets::generated::misc::play_sound::{
    PlaySoundPacketAny, PlaySoundPacketV897,
};
use common::bedrock_protocol::protocol::types::primitives::BlockPos;
use common::structs::control::ClientActionType;
use common::structs::packet::{BedrockEvent, PacketType, QuicNetworkPacketData};

fn make_emitter() -> (Arc<BedrockEventEmitter>, flume::Receiver<NetworkPacket>) {
    let (tx, rx) = flume::unbounded::<NetworkPacket>();
    (Arc::new(BedrockEventEmitter::new(Arc::new(tx))), rx)
}

fn packet(name: &str, pos: BlockPos) -> PlaySoundPacketAny {
    PlaySoundPacketAny::V897(PlaySoundPacketV897 {
        name: name.to_string(),
        position: pos,
        volume: 1.0,
        pitch: 1.0,
    })
}

// Drive a PlaySound through the real handler and return whatever it queued to the
// serverbound egress (or None if it was ignored). Tests observe the handler purely
// through its public output rather than its private `parse` helper.
fn drive(name: &str, pos: BlockPos, player: &str, world: Option<&str>) -> Option<NetworkPacket> {
    let (emitter, rx) = make_emitter();
    let beacon_cache = JukeboxBeaconCache::default();
    let mut state = BedrockSessionState::new(player.to_string(), Some("xuid-1".to_string()));
    if let Some(w) = world {
        state.set_world_uuid_for_test(w.to_string());
    }
    PlaySoundHandler {
        beacon_cache: &beacon_cache,
        player_name: player,
    }
    .handle(&packet(name, pos), &mut state, Some(&emitter));
    rx.try_recv().ok()
}

#[test]
fn jukebox_insert_carries_parsed_coords_audio_id_and_world_uuid() {
    let queued = drive(
        "bvc:play:019d1701-7bb8-7e70-9a36-65653d22245d:minecraft:overworld",
        // Bedrock 1/8-block fixed point: wire value is world_coord * 8.
        BlockPos::new(976, 1072, 192),
        "Alice",
        Some("world-uuid-xyz"),
    )
    .expect("JukeboxInsert should be queued");

    assert_eq!(queued.data.packet_type, PacketType::BedrockEvent);
    match queued.data.data {
        QuicNetworkPacketData::BedrockEvent(ep) => match ep.event {
            BedrockEvent::JukeboxInsert {
                audio_id,
                block_pos,
                relay_world_uuid,
                ..
            } => {
                assert_eq!(audio_id, "019d1701-7bb8-7e70-9a36-65653d22245d");
                assert_eq!(block_pos.x, 122.0);
                assert_eq!(block_pos.y, 134.0);
                assert_eq!(block_pos.z, 24.0);
                assert_eq!(relay_world_uuid, Some("world-uuid-xyz".to_string()));
            }
            other => panic!("expected JukeboxInsert, got {:?}", other),
        },
        other => panic!("expected BedrockEvent packet, got {:?}", other),
    }
}

#[test]
fn jukebox_insert_recovers_negative_coords_via_fixed_point_division() {
    let queued = drive(
        "bvc:play:track:minecraft:nether",
        BlockPos::new(-56, 8, -8),
        "Bob",
        Some("world-1"),
    )
    .expect("JukeboxInsert should be queued");

    match queued.data.data {
        QuicNetworkPacketData::BedrockEvent(ep) => match ep.event {
            BedrockEvent::JukeboxInsert { block_pos, .. } => {
                assert_eq!(block_pos.x, -7.0);
                assert_eq!(block_pos.y, 1.0);
                assert_eq!(block_pos.z, -1.0);
            }
            other => panic!("expected JukeboxInsert, got {:?}", other),
        },
        other => panic!("expected BedrockEvent packet, got {:?}", other),
    }
}

#[test]
fn bvc_ctl_emits_serverbound_client_action_for_the_session_player() {
    let queued = drive("bvc:ctl:mute:1", BlockPos::new(0, 0, 0), "Alice", Some("world-1"))
        .expect("ClientAction should be queued");

    assert_eq!(queued.data.packet_type, PacketType::ClientAction);
    match queued.data.data {
        QuicNetworkPacketData::ClientAction(p) => {
            assert_eq!(p.action.id, "Alice");
            assert_eq!(p.action.action, ClientActionType::SetMuted(true));
        }
        other => panic!("expected ClientAction, got {:?}", other),
    }
}

#[test]
fn ignores_non_bvc_and_dimensionless_play_names() {
    assert!(
        drive("random.sound", BlockPos::new(0, 0, 0), "Alice", Some("world-1")).is_none(),
        "a non-bvc sound must not be intercepted"
    );
    assert!(
        drive("bvc:play:abc", BlockPos::new(0, 0, 0), "Alice", Some("world-1")).is_none(),
        "bvc:play without a dimension is malformed and must be ignored"
    );
}
