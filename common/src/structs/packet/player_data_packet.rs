use serde::de::Deserializer;
use serde::ser::{SerializeStruct, Serializer};
use serde::{Deserialize, Serialize};

use super::quic_network_packet_data::QuicNetworkPacketData;
use crate::PlayerEnum;

#[derive(Debug, Clone)]
pub struct PlayerDataPacket {
    pub players: Vec<crate::PlayerEnum>,
}

// Compact wire form: `world_uuid` is lifted out of the players and written once.
// A position payload covers a single world, so the identifier was otherwise
// repeated per player -- 38 bytes each for a hyphenated UUID, 66 for a blake3
// digest -- which is what pushed multi-player packets past MAX_DATAGRAM_SIZE.
#[derive(Deserialize)]
struct HoistedWire {
    world_uuid: Option<String>,
    players: Vec<PlayerEnum>,
}

#[derive(Deserialize)]
struct PlainWire {
    players: Vec<PlayerEnum>,
}

impl PlayerDataPacket {
    pub fn new(players: Vec<PlayerEnum>) -> Self {
        Self { players }
    }

    /// The world identifier the compact wire form will hoist out of `players`,
    /// or `None` when the packet will carry each player's own.
    ///
    /// Hoisting is only lossless when every player agrees on a world, so a
    /// single `None` -- a `Generic` player, or a genuinely world-less entry --
    /// suppresses it rather than inventing a world on the way back in.
    pub fn shared_world_uuid(players: &[PlayerEnum]) -> Option<String> {
        let mut worlds = players.iter().map(PlayerEnum::world_uuid);
        let first = worlds.next().flatten()?;
        worlds
            .all(|world| world == Some(first))
            .then(|| first.to_string())
    }
}

impl Serialize for PlayerDataPacket {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        // JSON and RON keep the original single-field shape, so debug output
        // and any human-facing surface are untouched by the wire optimisation.
        if serializer.is_human_readable() {
            let mut state = serializer.serialize_struct("PlayerDataPacket", 1)?;
            state.serialize_field("players", &self.players)?;
            return state.end();
        }

        let shared = Self::shared_world_uuid(&self.players);
        let mut state = serializer.serialize_struct("PlayerDataPacket", 2)?;
        state.serialize_field("world_uuid", &shared)?;

        match shared {
            Some(_) => {
                let stripped: Vec<PlayerEnum> = self
                    .players
                    .iter()
                    .map(|player| {
                        let mut player = player.clone();
                        player.set_world_uuid(None);
                        player
                    })
                    .collect();
                state.serialize_field("players", &stripped)?;
            }
            None => state.serialize_field("players", &self.players)?,
        }

        state.end()
    }
}

impl<'de> Deserialize<'de> for PlayerDataPacket {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        if deserializer.is_human_readable() {
            let wire = PlainWire::deserialize(deserializer)?;
            return Ok(Self {
                players: wire.players,
            });
        }

        let wire = HoistedWire::deserialize(deserializer)?;
        let mut players = wire.players;

        if let Some(world_uuid) = wire.world_uuid {
            for player in &mut players {
                player.set_world_uuid(Some(world_uuid.clone()));
            }
        }

        Ok(Self { players })
    }
}

impl TryFrom<QuicNetworkPacketData> for PlayerDataPacket {
    type Error = ();

    fn try_from(value: QuicNetworkPacketData) -> Result<Self, Self::Error> {
        match value {
            QuicNetworkPacketData::PlayerData(c) => Ok(c),
            _ => Err(()),
        }
    }
}
