use crate::errors::CommunicationError;
use crate::game_data::Dimension;
use crate::players::{GenericPlayer, MinecraftPlayer};
use crate::traits::player_data::{PlayerData, SpatialPlayer};
use crate::{Coordinate, Game, Orientation};
use serde::{Deserialize, Serialize};

// Inert values for the Reserved slot. Returned rather than panicked on, because Reserved is
// reachable by decoding a datagram from a build that still sends a player at index 1, and a
// panic there lands on the QUIC hot path.
static RESERVED_COORDINATE: Coordinate = Coordinate {
    x: 0.0,
    y: 0.0,
    z: 0.0,
};
static RESERVED_ORIENTATION: Orientation = Orientation { x: 0.0, y: 0.0 };

/// Type-safe enum for storing heterogeneous player types
/// Dispatches to game-specific implementations
///
/// Uses externally tagged serialization (works with postcard).
/// For JSON compatibility with legacy clients, use GameDataCollection's
/// custom deserialization which handles the game field at the collection level.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
pub enum PlayerEnum {
    Minecraft(MinecraftPlayer),
    // Holds postcard index 1 so Generic keeps index 2. Produced only by decoding a datagram
    // from a build that still sends a player here; it carries no data and is dropped where
    // players enter the position path.
    Reserved,
    Generic(GenericPlayer),
}

impl PlayerData for PlayerEnum {
    fn get_name(&self) -> &str {
        match self {
            PlayerEnum::Minecraft(p) => p.get_name(),
            PlayerEnum::Reserved => "",
            PlayerEnum::Generic(p) => p.get_name(),
        }
    }

    fn get_position(&self) -> &Coordinate {
        match self {
            PlayerEnum::Minecraft(p) => p.get_position(),
            PlayerEnum::Reserved => &RESERVED_COORDINATE,
            PlayerEnum::Generic(p) => p.get_position(),
        }
    }

    fn get_orientation(&self) -> &Orientation {
        match self {
            PlayerEnum::Minecraft(p) => p.get_orientation(),
            PlayerEnum::Reserved => &RESERVED_ORIENTATION,
            PlayerEnum::Generic(p) => p.get_orientation(),
        }
    }

    // Deafened, so anything that reaches this before the ingestion filter treats it as
    // somebody who receives no audio rather than as a silent listener.
    fn is_deafened(&self) -> bool {
        match self {
            PlayerEnum::Minecraft(p) => p.is_deafened(),
            PlayerEnum::Reserved => true,
            PlayerEnum::Generic(p) => p.is_deafened(),
        }
    }

    fn get_game(&self) -> Game {
        match self {
            PlayerEnum::Minecraft(p) => p.get_game(),
            PlayerEnum::Reserved => Game::Minecraft,
            PlayerEnum::Generic(p) => p.get_game(),
        }
    }

    fn world_identifier(&self) -> Option<&str> {
        match self {
            PlayerEnum::Minecraft(p) => p.world_identifier(),
            PlayerEnum::Reserved => None,
            PlayerEnum::Generic(p) => p.world_identifier(),
        }
    }

    fn has_bridged_voice(&self) -> bool {
        match self {
            PlayerEnum::Minecraft(p) => p.has_bridged_voice(),
            PlayerEnum::Reserved => false,
            PlayerEnum::Generic(p) => p.has_bridged_voice(),
        }
    }

    fn dimension(&self) -> Option<Dimension> {
        match self {
            PlayerEnum::Minecraft(p) => p.dimension(),
            PlayerEnum::Reserved => None,
            PlayerEnum::Generic(p) => p.dimension(),
        }
    }

    fn clone_box(&self) -> Box<dyn PlayerData> {
        Box::new(self.clone())
    }
}

impl SpatialPlayer for PlayerEnum {}

impl PlayerEnum {
    /// Dispatch to game-specific can_communicate_with implementation
    /// Each game type knows how to handle its own spatial logic
    pub fn can_communicate_with(
        &self,
        other: &PlayerEnum,
        range: f32,
    ) -> Result<(), CommunicationError> {
        // Players from different games can't communicate
        if self.get_game() != other.get_game() {
            return Err(CommunicationError::GameMismatch {
                sender_game: self.get_game(),
                recipient_game: other.get_game(),
            });
        }

        // Dispatch to the game-specific implementation. Agreeing on the game does not imply
        // agreeing on the variant: Reserved and a Generic player carrying Game::Minecraft
        // both report Minecraft, so a mismatched pair is out of scope rather than impossible.
        match self {
            PlayerEnum::Minecraft(mc_self) => {
                if let PlayerEnum::Minecraft(mc_other) = other {
                    mc_self.can_communicate_with(mc_other, range)
                } else {
                    Err(CommunicationError::OutOfRange {
                        distance: f32::INFINITY,
                        max_range: range,
                    })
                }
            }
            PlayerEnum::Reserved => Err(CommunicationError::OutOfRange {
                distance: f32::INFINITY,
                max_range: range,
            }),
            PlayerEnum::Generic(gen_self) => {
                if let PlayerEnum::Generic(gen_other) = other {
                    gen_self.can_communicate_with(gen_other, range)
                } else {
                    Err(CommunicationError::OutOfRange {
                        distance: f32::INFINITY,
                        max_range: range,
                    })
                }
            }
        }
    }

    /// Helper to get Minecraft player if this is a Minecraft player
    pub fn as_minecraft(&self) -> Option<&MinecraftPlayer> {
        match self {
            PlayerEnum::Minecraft(mc) => Some(mc),
            _ => None,
        }
    }

    /// Whether this is the reserved slot rather than a player.
    ///
    /// True only for a value decoded from a build that still sends a player at postcard
    /// index 1. Callers that place players in the world drop these instead of indexing them,
    /// because the slot carries no position, name or game to index on.
    pub fn is_reserved(&self) -> bool {
        matches!(self, PlayerEnum::Reserved)
    }

    /// Helper to get Generic player if this is a Generic player
    pub fn as_generic(&self) -> Option<&GenericPlayer> {
        match self {
            PlayerEnum::Generic(g) => Some(g),
            _ => None,
        }
    }

    /// Get the alternative identity if one exists (e.g., Xbox gamertag for Floodgate players)
    pub fn get_alternative_identity(&self) -> Option<&str> {
        match self {
            PlayerEnum::Minecraft(mc) => mc.alternative_identity.as_deref(),
            _ => None,
        }
    }

    /// Get the platform UUID if one exists (e.g., Minecraft Java UUID)
    pub fn get_player_uuid(&self) -> Option<&str> {
        match self {
            PlayerEnum::Minecraft(mc) => mc.player_uuid.as_deref(),
            PlayerEnum::Reserved => None,
            PlayerEnum::Generic(_) => None,
        }
    }

    /// Set the player name (used for identity resolution)
    pub fn set_name(&mut self, name: String) {
        match self {
            PlayerEnum::Minecraft(mc) => mc.name = name,
            PlayerEnum::Reserved => {}
            PlayerEnum::Generic(g) => g.name = name,
        }
    }

    /// World identity, where the variant carries one. `Generic` has no world
    /// concept and always reports `None`.
    ///
    /// The value is an opaque identifier, not a parseable UUID: the BDS mod
    /// publishes a hyphenated UUIDv4 while the client proxy publishes the
    /// 64-character blake3 digest from [`crate::structs::bedrock::BedrockWorldId`].
    pub fn world_uuid(&self) -> Option<&str> {
        match self {
            PlayerEnum::Minecraft(mc) => mc.world_uuid.as_deref(),
            PlayerEnum::Reserved => None,
            PlayerEnum::Generic(_) => None,
        }
    }

    /// Overwrite world identity. A no-op for `Generic`, which has no field.
    pub fn set_world_uuid(&mut self, world_uuid: Option<String>) {
        match self {
            PlayerEnum::Minecraft(mc) => mc.world_uuid = world_uuid,
            PlayerEnum::Reserved => {}
            PlayerEnum::Generic(_) => {}
        }
    }
}
