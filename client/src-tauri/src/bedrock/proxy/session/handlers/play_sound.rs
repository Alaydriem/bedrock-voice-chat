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

    pub fn parse(name: &str, position: &BlockPos) -> Option<JukeboxCommand> {
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
                info!("Bedrock proxy: bvc:eject -> JukeboxEject event_id={}", event_id);
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
