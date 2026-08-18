use crate::errors::CommunicationError;
use crate::game_data::Dimension;
use crate::players::{GenericPlayer, HytalePlayer, MinecraftPlayer};
use crate::traits::player_data::{PlayerData, SpatialPlayer};
use crate::{Coordinate, Game, Orientation};
use serde::{Deserialize, Serialize};

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
    Hytale(HytalePlayer),
    Generic(GenericPlayer),
}

impl PlayerData for PlayerEnum {
    fn get_name(&self) -> &str {
        match self {
            PlayerEnum::Minecraft(p) => p.get_name(),
            PlayerEnum::Hytale(p) => p.get_name(),
            PlayerEnum::Generic(p) => p.get_name(),
        }
    }

    fn get_position(&self) -> &Coordinate {
        match self {
            PlayerEnum::Minecraft(p) => p.get_position(),
            PlayerEnum::Hytale(p) => p.get_position(),
            PlayerEnum::Generic(p) => p.get_position(),
        }
    }

    fn get_orientation(&self) -> &Orientation {
        match self {
            PlayerEnum::Minecraft(p) => p.get_orientation(),
            PlayerEnum::Hytale(p) => p.get_orientation(),
            PlayerEnum::Generic(p) => p.get_orientation(),
        }
    }

    fn is_deafened(&self) -> bool {
        match self {
            PlayerEnum::Minecraft(p) => p.is_deafened(),
            PlayerEnum::Hytale(p) => p.is_deafened(),
            PlayerEnum::Generic(p) => p.is_deafened(),
        }
    }

    fn get_game(&self) -> Game {
        match self {
            PlayerEnum::Minecraft(p) => p.get_game(),
            PlayerEnum::Hytale(p) => p.get_game(),
            PlayerEnum::Generic(p) => p.get_game(),
        }
    }

    fn world_identifier(&self) -> Option<&str> {
        match self {
            PlayerEnum::Minecraft(p) => p.world_identifier(),
            PlayerEnum::Hytale(p) => p.world_identifier(),
            PlayerEnum::Generic(p) => p.world_identifier(),
        }
    }

    fn has_bridged_voice(&self) -> bool {
        match self {
            PlayerEnum::Minecraft(p) => p.has_bridged_voice(),
            PlayerEnum::Hytale(p) => p.has_bridged_voice(),
            PlayerEnum::Generic(p) => p.has_bridged_voice(),
        }
    }

    fn dimension(&self) -> Option<Dimension> {
        match self {
            PlayerEnum::Minecraft(p) => p.dimension(),
            PlayerEnum::Hytale(p) => p.dimension(),
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

        // Dispatch to the game-specific implementation
        match self {
            PlayerEnum::Minecraft(mc_self) => {
                if let PlayerEnum::Minecraft(mc_other) = other {
                    mc_self.can_communicate_with(mc_other, range)
                } else {
                    unreachable!("Game mismatch already checked above")
                }
            }
            PlayerEnum::Hytale(hy_self) => {
                if let PlayerEnum::Hytale(hy_other) = other {
                    hy_self.can_communicate_with(hy_other, range)
                } else {
                    unreachable!("Game mismatch already checked above")
                }
            }
            PlayerEnum::Generic(gen_self) => {
                if let PlayerEnum::Generic(gen_other) = other {
                    gen_self.can_communicate_with(gen_other, range)
                } else {
                    unreachable!("Game mismatch already checked above")
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

    /// Helper to get Hytale player if this is a Hytale player
    pub fn as_hytale(&self) -> Option<&HytalePlayer> {
        match self {
            PlayerEnum::Hytale(h) => Some(h),
            _ => None,
        }
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

    /// Get the platform UUID if one exists (e.g., Hytale UUID, Minecraft Java UUID)
    pub fn get_player_uuid(&self) -> Option<&str> {
        match self {
            PlayerEnum::Minecraft(mc) => mc.player_uuid.as_deref(),
            PlayerEnum::Hytale(h) => h.player_uuid.as_deref(),
            PlayerEnum::Generic(_) => None,
        }
    }

    /// Set the player name (used for identity resolution)
    pub fn set_name(&mut self, name: String) {
        match self {
            PlayerEnum::Minecraft(mc) => mc.name = name,
            PlayerEnum::Hytale(h) => h.name = name,
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
            PlayerEnum::Hytale(h) => h.world_uuid.as_deref(),
            PlayerEnum::Generic(_) => None,
        }
    }

    /// Overwrite world identity. A no-op for `Generic`, which has no field.
    pub fn set_world_uuid(&mut self, world_uuid: Option<String>) {
        match self {
            PlayerEnum::Minecraft(mc) => mc.world_uuid = world_uuid,
            PlayerEnum::Hytale(h) => h.world_uuid = world_uuid,
            PlayerEnum::Generic(_) => {}
        }
    }
}
