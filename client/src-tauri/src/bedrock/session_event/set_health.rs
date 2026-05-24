use std::sync::Arc;

use common::bedrock_protocol::protocol::packets::generated::misc::set_health::SetHealthPacket;
use common::structs::packet::BedrockEvent;
use log::info;

use crate::bedrock::event_emitter::BedrockEventEmitter;
use crate::bedrock::session_event::BedrockPacketHandler;
use crate::bedrock::session_state::BedrockSessionState;

pub struct SetHealthHandler<'a> {
    pub last_known_health: &'a mut Option<i32>,
}

impl<'a> BedrockPacketHandler for SetHealthHandler<'a> {
    type Packet = SetHealthPacket;

    fn handle(
        self,
        packet: &SetHealthPacket,
        state: &mut BedrockSessionState,
        emitter: Option<&Arc<BedrockEventEmitter>>,
    ) {
        let new_health = packet.health;
        let previously_alive = matches!(self.last_known_health, Some(h) if *h > 0);
        *self.last_known_health = Some(new_health);

        if new_health > 0 || !previously_alive {
            return;
        }

        let emitter = match emitter {
            Some(e) => e,
            None => return,
        };
        let world_uuid = match state.world_uuid() {
            Some(w) => w.to_string(),
            None => return,
        };

        let event = BedrockEvent::PlayerDeath {
            player_xuid: state.player_uuid().unwrap_or("").to_string(),
            dimension: state.dimension(),
            last_pos: state.coordinates(),
        };
        info!("Bedrock proxy: emitting player death for {}", state.name());
        emitter.try_send(event, world_uuid);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(prev: Option<i32>, new_health: i32) -> Option<i32> {
        let mut last = prev;
        let mut state = BedrockSessionState::new("tester".into(), Some("xuid".into()));
        let packet = SetHealthPacket { health: new_health };
        SetHealthHandler {
            last_known_health: &mut last,
        }
        .handle(&packet, &mut state, None);
        last
    }

    #[test]
    fn not_yet_alive_does_not_emit_or_panic() {
        assert_eq!(run(None, 0), Some(0));
        assert_eq!(run(None, 20), Some(20));
    }

    #[test]
    fn alive_to_dead_updates_last_known() {
        assert_eq!(run(Some(20), 0), Some(0));
    }

    #[test]
    fn already_dead_stays_dead() {
        assert_eq!(run(Some(0), 0), Some(0));
    }

    #[test]
    fn dead_to_alive_updates_last_known() {
        assert_eq!(run(Some(0), 20), Some(20));
    }
}
