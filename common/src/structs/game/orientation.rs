use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Orientation {
    pub x: f32,
    pub y: f32,
}
