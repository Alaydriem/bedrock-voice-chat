use std::sync::Arc;

use bvc_client_lib::NetworkPacket;
use bvc_client_lib::bedrock::BedrockEventEmitter;
use bvc_client_lib::bedrock::JukeboxBeaconCache;
use bvc_client_lib::bedrock::proxy::session::handlers::PlaySoundHandler;
use bvc_client_lib::bedrock::proxy::session::{BedrockPacketHandler, BedrockSessionState};
use bvc_client_lib::control::{ControlActionSender, ControlStateBus, ControlStateSignal};
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
    drive_with_control(name, pos, player, world).0
}

// Also returns the control-channel receiver and the state-bus receiver so tests
// can assert which plane a bvc:ctl message rode: ServerBound egress (group), the
// local control channel (self/preference), or the state bus (sync).
fn drive_with_control(
    name: &str,
    pos: BlockPos,
    player: &str,
    world: Option<&str>,
) -> (
    Option<NetworkPacket>,
    flume::Receiver<ClientActionType>,
    tokio::sync::broadcast::Receiver<ControlStateSignal>,
) {
    let (emitter, rx) = make_emitter();
    let (control_tx, control_rx) = ControlActionSender::channel();
    let state_bus = ControlStateBus::new();
    let bus_rx = state_bus.subscribe();
    let beacon_cache = JukeboxBeaconCache::default();
    let mut state = BedrockSessionState::new(player.to_string(), Some("xuid-1".to_string()));
    if let Some(w) = world {
        state.set_world_uuid_for_test(w.to_string());
    }
    PlaySoundHandler {
        beacon_cache: &beacon_cache,
        player_name: player,
        control_tx,
        state_bus,
    }
    .handle(&packet(name, pos), &mut state, Some(&emitter));
    (rx.try_recv().ok(), control_rx, bus_rx)
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
fn bvc_ctl_group_action_emits_serverbound_client_action_for_the_session_player() {
    // Group actions must reach the server, so they ride ServerBound (unchanged).
    let queued = drive(
        "bvc:ctl:group:create",
        BlockPos::new(0, 0, 0),
        "Alice",
        Some("world-1"),
    )
    .expect("group ClientAction should be queued");

    assert_eq!(queued.data.packet_type, PacketType::ClientAction);
    match queued.data.data {
        QuicNetworkPacketData::ClientAction(p) => {
            assert_eq!(p.action.id, "Alice");
            assert_eq!(p.action.action, ClientActionType::CreateGroup);
        }
        other => panic!("expected ClientAction, got {:?}", other),
    }
}

#[test]
fn bvc_ctl_self_action_rides_the_control_channel_not_serverbound() {
    // Self/preference actions are applied LOCALLY (the no-net shortcut): the handler
    // pushes them onto the control channel for the app-level consumer and never
    // emits ServerBound. The end-to-end apply effect is covered by the
    // `control_mute` client-e2e scenario.
    let (queued, control_rx, _) = drive_with_control(
        "bvc:ctl:mute:1",
        BlockPos::new(0, 0, 0),
        "Alice",
        Some("world-1"),
    );

    assert!(
        queued.is_none(),
        "a self control action must not emit a ServerBound ClientAction"
    );
    assert_eq!(
        control_rx.try_recv().ok(),
        Some(ClientActionType::SetMuted(true)),
        "the self action must be pushed onto the local control channel"
    );
}

#[test]
fn ignores_non_bvc_and_dimensionless_play_names() {
    assert!(
        drive(
            "random.sound",
            BlockPos::new(0, 0, 0),
            "Alice",
            Some("world-1")
        )
        .is_none(),
        "a non-bvc sound must not be intercepted"
    );
    assert!(
        drive(
            "bvc:play:abc",
            BlockPos::new(0, 0, 0),
            "Alice",
            Some("world-1")
        )
        .is_none(),
        "bvc:play without a dimension is malformed and must be ignored"
    );
}

#[test]
fn bvc_ctl_sync_arms_the_session_for_bvcs_rides() {
    // Only the BVC addon's panel emits sync; its arrival is the proof that
    // !bvcs: rides are safe to inject into this session (something will cancel
    // them). Without it the session must stay unarmed.
    let (emitter, _rx) = make_emitter();
    let (control_tx, _control_rx) = ControlActionSender::channel();
    let beacon_cache = JukeboxBeaconCache::default();
    let mut state = BedrockSessionState::new("Alice".to_string(), Some("xuid-1".to_string()));
    state.set_world_uuid_for_test("world-1".to_string());
    assert!(!state.bvcs_armed(), "a fresh session must start unarmed");

    PlaySoundHandler {
        beacon_cache: &beacon_cache,
        player_name: "Alice",
        control_tx: control_tx.clone(),
        state_bus: ControlStateBus::new(),
    }
    .handle(
        &packet("bvc:ctl:mute:1", BlockPos::new(0, 0, 0)),
        &mut state,
        Some(&emitter),
    );
    assert!(
        !state.bvcs_armed(),
        "a self action must not arm the session — only a sync proves the addon exists"
    );

    PlaySoundHandler {
        beacon_cache: &beacon_cache,
        player_name: "Alice",
        control_tx,
        state_bus: ControlStateBus::new(),
    }
    .handle(
        &packet("bvc:ctl:sync:", BlockPos::new(0, 0, 0)),
        &mut state,
        Some(&emitter),
    );
    assert!(state.bvcs_armed(), "a decoded sync must arm the session");
}

#[test]
fn bvc_ctl_sync_signals_the_state_bus_with_scoped_targets() {
    // A panel snapshot request is neither a ClientAction nor a local apply — it
    // rides the state bus so the reporter answers with !bvcs: messages.
    let (queued, control_rx, mut bus_rx) = drive_with_control(
        "bvc:ctl:sync:Bob,Carl",
        BlockPos::new(0, 0, 0),
        "Alice",
        Some("world-1"),
    );

    assert!(queued.is_none(), "sync must not emit ServerBound");
    assert!(
        control_rx.try_recv().is_err(),
        "sync must not ride the control-action channel"
    );
    assert_eq!(
        bus_rx.try_recv().ok(),
        Some(ControlStateSignal::Sync {
            targets: vec!["Bob".to_string(), "Carl".to_string()],
        }),
        "sync must signal the state bus with its scoped targets"
    );
}
