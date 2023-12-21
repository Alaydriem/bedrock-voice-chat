pub mod ncryptflib;
pub use ncryptf::{
    rocket_db_pools, rocket_dyn_templates, rocketfw as rocket, sea_orm, sea_orm_migration,
    sea_orm_rocket,
};
pub mod auth;
pub mod pool;
pub mod redis;
pub use serde::{Deserialize, Serialize};

pub mod consts;
pub mod structs;

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
