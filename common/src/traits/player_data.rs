use crate::game_data::Dimension;
use crate::{Coordinate, Game, Orientation};

/// Core trait - ALL players implement this
pub trait PlayerData: Send + Sync {
    fn get_name(&self) -> &str;
    fn get_position(&self) -> &Coordinate;
    fn get_orientation(&self) -> &Orientation;
    fn is_deafened(&self) -> bool {
        false
    }
    fn get_game(&self) -> Game;
    fn clone_box(&self) -> Box<dyn PlayerData>;

    /// The canonical identity this player is keyed on everywhere: `game:gamertag`.
    ///
    /// Derived rather than stored, because the game is already the variant tag and the bare
    /// name is already the `name` field. Holding the composed form as a third piece of state
    /// is what let the two forms drift apart.
    ///
    /// This is the only place a canonical identity is produced in Rust. `Game::membership_key`
    /// is its equivalent for callers that hold the game and the name loose.
    fn identity(&self) -> String {
        self.get_game().membership_key(self.get_name())
    }

    /// The world this player is in, as the identifier cross-server peering scopes on.
    ///
    /// Each game answers for itself: Minecraft's is the mod-supplied `relay_world_uuid`,
    /// which is deliberately not a Minecraft world UUID. `None` means the player cannot
    /// be scoped to a world, so peering has no question to which "yes" is a safe answer.
    ///
    /// On the trait rather than on `Game`, because `Game` is only the variant tag — the
    /// value itself lives on the player. Callers must never reach for a concrete variant
    /// to find it; doing so is what silently confined peering to one game.
    fn world_identifier(&self) -> Option<&str> {
        None
    }

    /// This player's dimension, as the type anything positional is placed against.
    ///
    /// `None` means the player cannot be placed in one — either the game has no
    /// notion of a dimension, or it has its own type that is not this one. Hytale
    /// is the second case: it carries `game_data::hytale::Dimension`, a different
    /// type, so it answers `None` here rather than pretending to convert.
    ///
    /// On the trait for the same reason as `world_identifier` — a caller reaching
    /// for a concrete variant to read it is how a behaviour ends up quietly
    /// confined to one game.
    fn dimension(&self) -> Option<Dimension> {
        None
    }
}

/// Spatial communication trait
/// Provides default distance calculation
pub trait SpatialPlayer: PlayerData {
    /// Calculate 3D Euclidean distance to another player
    fn distance_to(&self, other: &dyn PlayerData) -> f32 {
        let my_pos = self.get_position();
        let other_pos = other.get_position();

        let dx = my_pos.x - other_pos.x;
        let dy = my_pos.y - other_pos.y;
        let dz = my_pos.z - other_pos.z;

        (dx * dx + dy * dy + dz * dz).sqrt()
    }
}
