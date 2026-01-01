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
