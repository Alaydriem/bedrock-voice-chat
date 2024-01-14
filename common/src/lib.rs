pub mod ncryptflib;

pub mod auth;
pub mod pool;
pub mod redis;
pub use serde::{ Deserialize, Serialize };
pub mod certificates;

pub mod consts;
pub mod structs;
pub mod mtlsprovider;

extern crate rocket;

pub use rocket::time::OffsetDateTime as RocketOffsetDateTime;
pub use rocket::time::Duration as RocketDuration;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Dimension {
    #[serde(rename = "overworld")]
    Overworld,
    #[serde(rename = "the_nether")]
    TheEnd,
    #[serde(rename = "the_end")]
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
