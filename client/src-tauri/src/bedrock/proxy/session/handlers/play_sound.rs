use std::sync::Arc;

use common::bedrock_protocol::protocol::packets::generated::misc::play_sound::PlaySoundPacketAny;
use common::bedrock_protocol::protocol::types::primitives::BlockPos;
use common::game_data::Dimension;
use common::structs::game::{BlockCoordinate, Coordinate};
use common::structs::packet::BedrockEvent;
use log::{debug, info};

use super::JukeboxCommand;
use common::structs::control::{ClientAction, CtlCodec, CtlMessage};

use crate::bedrock::BedrockEventEmitter;
use crate::bedrock::JukeboxBeaconCache;
use crate::bedrock::proxy::session::BedrockPacketHandler;
use crate::bedrock::proxy::session::BedrockSessionState;

// Wire contract with the BDS mod's NoNetAudioSender (mods/bds/src/audio/sender/protocol.ts).
// Keep these prefixes in sync with JukeboxBusProtocol.PLAY / EJECT.
const PLAY: &str = "bvc:play:";
const EJECT: &str = "bvc:eject:";
// Control-plane action bus. The action grammar + decode live in common's CtlCodec.
const CTL: &str = "bvc:ctl:";

pub struct PlaySoundHandler<'a> {
    pub beacon_cache: &'a JukeboxBeaconCache,
    pub player_name: &'a str,
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
            PlaySoundPacketAny::V2169(p) => &p.position,
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

        let name = packet.name();
        // bvc:ctl:<action> is a control-plane action, not a sound: emit it to the BVC
        // server as a ServerBound ClientAction and forward nothing to the client. The
        // actor is this session's authenticated player.
        if name.starts_with(CTL) {
            if let Some(CtlMessage::Action(action)) = CtlCodec::decode(name) {
                emitter.try_send_client_action(ClientAction {
                    id: self.player_name.to_string(),
                    action,
                });
            }
            return;
        }

        let command = match Self::parse(name, Self::position(packet)) {
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
