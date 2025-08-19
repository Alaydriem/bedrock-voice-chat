pub mod ncryptflib;

pub mod auth;
pub mod encoding;
pub mod pool;
pub mod redis;
pub use serde::{Deserialize, Serialize};
pub mod certificates;

pub mod consts;
pub mod request;
pub mod rustls;
pub mod structs;
pub mod traits;

extern crate rocket;

pub use rocket::time::Duration as RocketDuration;
pub use rocket::time::OffsetDateTime as RocketOffsetDateTime;

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum Dimension {
    #[serde(rename = "overworld")]
    Overworld,
    #[serde(rename = "the_end")]
    TheEnd,
    #[serde(rename = "nether")]
    TheNether,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Coordinate {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Default for Coordinate {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Orientation {
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Player {
    pub name: String,
    pub dimension: Dimension,
    pub deafen: bool,
    pub coordinates: Coordinate,
    pub orientation: Orientation
}


