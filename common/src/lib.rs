pub mod ncryptflib;
pub use ncryptf::{rocket_db_pools, rocketfw as rocket};
pub mod auth;
pub mod pool;
pub mod redis;
pub use serde::{Deserialize, Serialize};

pub mod structs;
/// [{"name":"Alaydriem","dimension":"minecraft:overworld","coordinates":{"x":0.5,"y":70,"z":0.5},"deafen":false}]

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Dimension {
    #[serde(rename = "minecraft:overworld")]
    Overworld,
    #[serde(rename = "minecraft:the_nether")]
    TheEnd,
    #[serde(rename = "minecraft:the_end")]
    TheNether,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Coordinate {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Player {
    pub name: String,
    pub dimension: Dimension,
    pub deafen: bool,
    pub coordinates: Coordinate,
}