use std::sync::Arc;

use common::bedrock_protocol::protocol::packets::generated::misc::play_sound::PlaySoundPacketAny;
use common::bedrock_protocol::protocol::types::primitives::BlockPos;
use common::game_data::Dimension;
use common::structs::game::{BlockCoordinate, Coordinate};
use common::structs::packet::BedrockEvent;
use log::{debug, info};

use super::JukeboxCommand;
use crate::bedrock::BedrockEventEmitter;
use crate::bedrock::JukeboxBeaconCache;
use crate::bedrock::proxy::session::BedrockPacketHandler;
use crate::bedrock::proxy::session::BedrockSessionState;

// Wire contract with the BDS mod's NoNetAudioSender (mods/bds/src/audio/sender/protocol.ts).
// Keep these prefixes in sync with JukeboxBusProtocol.PLAY / EJECT.
const PLAY: &str = "bvc:play:";
const EJECT: &str = "bvc:eject:";

pub struct PlaySoundHandler<'a> {
    pub beacon_cache: &'a JukeboxBeaconCache,
}

impl<'a> PlaySoundHandler<'a> {
    // PlaySoundPacket.position is Bedrock 1/8-block fixed point: the wire value
    // is world_location * 8. Jukebox coords are integers, so /8 recovers them
    // exactly (including negatives).
    fn block_coords(position: &BlockPos) -> Coordinate {
        Coordinate {
            x: (position.x / 8) as f32,
            y: (position.y / 8) as f32,
            z: (position.z / 8) as f32,
        }
    }

    fn position(packet: &PlaySoundPacketAny) -> &BlockPos {
        match packet {
            PlaySoundPacketAny::V897(p) => &p.position,
            PlaySoundPacketAny::V944(p) => &p.position,
            PlaySoundPacketAny::V975(p) => &p.position,
        }
    }

    fn parse(name: &str, position: &BlockPos) -> Option<JukeboxCommand> {
        let pos = Self::block_coords(position);
        if let Some(rest) = name.strip_prefix(PLAY) {
            let (audio_id, dimension) = rest.split_once(':')?;
            return Some(JukeboxCommand::Play {
                audio_id: audio_id.to_string(),
                pos,
                dimension: dimension.to_string(),
            });
        }
        if let Some(dimension) = name.strip_prefix(EJECT) {
            return Some(JukeboxCommand::Eject {
                pos,
                dimension: dimension.to_string(),
            });
        }
        None
    }
}

impl<'a> BedrockPacketHandler for PlaySoundHandler<'a> {
    type Packet = PlaySoundPacketAny;

    fn handle(
        self,
        packet: &PlaySoundPacketAny,
        state: &mut BedrockSessionState,
        emitter: Option<&Arc<BedrockEventEmitter>>,
    ) {
        let emitter = match emitter {
            Some(e) => e,
            None => return,
        };
        let command = match Self::parse(packet.name(), Self::position(packet)) {
            Some(c) => c,
            None => return,
        };

        let world_uuid = match state.world_uuid() {
            Some(w) => w.to_string(),
            None => return,
        };
        let player_xuid = state.player_uuid().unwrap_or("").to_string();

        match command {
            JukeboxCommand::Play {
                audio_id,
                pos,
                dimension,
            } => {
                info!(
                    "Bedrock proxy: bvc:play -> JukeboxInsert audio_id={} at ({},{},{}) dim={}",
                    audio_id, pos.x, pos.y, pos.z, dimension
                );
                emitter.try_send(
                    BedrockEvent::JukeboxInsert {
                        audio_id,
                        block_pos: pos,
                        dimension: Dimension::from(dimension.as_str()),
                        player_xuid,
                        relay_world_uuid: Some(world_uuid.clone()),
                    },
                    world_uuid,
                );
            }
            JukeboxCommand::Eject { pos, dimension } => {
                let block_pos = BlockCoordinate::from(&pos);
                let dim = Dimension::from(dimension.as_str());
                let event_id = match self.beacon_cache.resolve_for_eject(block_pos, dim) {
                    Some(id) => id,
                    None => {
                        debug!(
                            "Bedrock proxy: bvc:eject no live beacon at ({},{},{}) dim={}; ignoring",
                            pos.x, pos.y, pos.z, dimension
                        );
                        return;
                    }
                };
                info!(
                    "Bedrock proxy: bvc:eject -> JukeboxEject event_id={}",
                    event_id
                );
                emitter.try_send(
                    BedrockEvent::JukeboxEject {
                        event_id,
                        player_xuid,
                    },
                    world_uuid,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network::NetworkPacket;
    use common::structs::packet::{PacketType, QuicNetworkPacketData};

    fn make_emitter() -> (Arc<BedrockEventEmitter>, flume::Receiver<NetworkPacket>) {
        let (tx, rx) = flume::unbounded::<NetworkPacket>();
        let emitter = Arc::new(BedrockEventEmitter::new(Arc::new(tx)));
        (emitter, rx)
    }

    #[test]
    fn jukebox_insert_emits_relay_world_uuid_matching_session_world_uuid() {
        let (emitter, rx) = make_emitter();
        let beacon_cache = JukeboxBeaconCache::default();

        let mut state =
            BedrockSessionState::new("TestPlayer".to_string(), Some("xuid-1".to_string()));
        state.set_world_uuid_for_test("world-uuid-xyz".to_string());

        let packet = PlaySoundPacketAny::V897(
            common::bedrock_protocol::protocol::packets::generated::misc::play_sound::PlaySoundPacketV897 {
                name: "bvc:play:019d1701-7bb8-7e70-9a36-65653d22245d:minecraft:overworld".to_string(),
                position: BlockPos::new(976, 1072, 192),
                volume: 1.0,
                pitch: 1.0,
            },
        );

        PlaySoundHandler {
            beacon_cache: &beacon_cache,
        }
        .handle(&packet, &mut state, Some(&emitter));

        let net_packet = rx
            .try_recv()
            .expect("JukeboxInsert packet should be queued");
        assert_eq!(net_packet.data.packet_type, PacketType::BedrockEvent);
        match net_packet.data.data {
            QuicNetworkPacketData::BedrockEvent(ep) => match ep.event {
                BedrockEvent::JukeboxInsert {
                    relay_world_uuid, ..
                } => {
                    assert_eq!(relay_world_uuid, Some("world-uuid-xyz".to_string()));
                }
                other => panic!("expected JukeboxInsert, got {:?}", other),
            },
            other => panic!("expected BedrockEvent packet, got {:?}", other),
        }
    }

    #[test]
    fn parses_play_with_dimension() {
        let cmd = PlaySoundHandler::parse(
            "bvc:play:019d1701-7bb8-7e70-9a36-65653d22245d:minecraft:overworld",
            &BlockPos::new(976, 1072, 192),
        )
        .expect("should parse");
        match cmd {
            JukeboxCommand::Play {
                audio_id,
                pos,
                dimension,
            } => {
                assert_eq!(audio_id, "019d1701-7bb8-7e70-9a36-65653d22245d");
                assert_eq!(pos.x, 122.0);
                assert_eq!(pos.y, 134.0);
                assert_eq!(pos.z, 24.0);
                assert_eq!(dimension, "minecraft:overworld");
            }
            _ => panic!("expected Play"),
        }
    }

    #[test]
    fn parses_eject_with_dimension_and_negative_coords() {
        let cmd = PlaySoundHandler::parse("bvc:eject:minecraft:nether", &BlockPos::new(-56, 8, -8))
            .expect("should parse");
        match cmd {
            JukeboxCommand::Eject { pos, dimension } => {
                assert_eq!(pos.x, -7.0);
                assert_eq!(pos.y, 1.0);
                assert_eq!(pos.z, -1.0);
                assert_eq!(dimension, "minecraft:nether");
            }
            _ => panic!("expected Eject"),
        }
    }

    #[test]
    fn rejects_non_bvc_name() {
        assert!(PlaySoundHandler::parse("random.sound", &BlockPos::new(0, 0, 0)).is_none());
    }

    #[test]
    fn rejects_play_without_dimension() {
        assert!(PlaySoundHandler::parse("bvc:play:abc", &BlockPos::new(0, 0, 0)).is_none());
    }
}
