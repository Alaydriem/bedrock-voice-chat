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
