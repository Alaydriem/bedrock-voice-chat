use common::{
    ncryptflib::rocket::JsonResponse,
    pool::redis::RedisDb,
    rocket::http::Status,
    rocket::response::status,
    rocket::serde::{Deserialize, json::Json},
    rocket_db_pools::Connection as RedisConnection,
};

#[allow(unused_imports)] // for rust-analyzer
use common::rocket_db_pools::deadpool_redis::redis::AsyncCommands;
use crate::rs::guards::MCAccessToken;

/// Stores player position data and online status from Minecraft Bedrock into Redis
#[post("/", data = "<positions>")]
pub async fn position(
    positions: Json<Vec::<common::Player>>,
    at: MCAccessToken,
    rdb: RedisConnection<RedisDb>,
) -> Status {

    return Status::Ok;
}