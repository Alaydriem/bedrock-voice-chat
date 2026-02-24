use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Coordinate {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Default for Coordinate {
    fn default() -> Self {
        Self { x: 0.0, y: 0.0, z: 0.0 }
    }
}
