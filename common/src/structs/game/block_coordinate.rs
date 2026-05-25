use serde::{Deserialize, Serialize};

use crate::structs::game::Coordinate;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
pub struct BlockCoordinate {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl BlockCoordinate {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }
}

impl From<BlockCoordinate> for Coordinate {
    fn from(b: BlockCoordinate) -> Self {
        Coordinate {
            x: b.x as f32,
            y: b.y as f32,
            z: b.z as f32,
        }
    }
}

impl From<&Coordinate> for BlockCoordinate {
    fn from(c: &Coordinate) -> Self {
        Self {
            x: c.x.floor() as i32,
            y: c.y.floor() as i32,
            z: c.z.floor() as i32,
        }
    }
}
