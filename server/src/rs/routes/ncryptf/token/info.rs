#[allow(unused_imports)] // for rust-analyzer
use rocket_db_pools::deadpool_redis::redis::AsyncCommands;

use rocket::http::Status;
use rocket_db_pools::Connection as RedisConnection;

use common::{ ncryptflib as ncryptf, pool::redis::RedisDb };
use entity::player::{ self, Model };
use ncryptf::rocket::Json;

/// This endpoint returns the curent token information
/// In practice, this is useful for validating that the user has a valid and functional access token
/// as this simply returns information the user already has.
#[get("/token/info")]
pub async fn token_info(
    _user: player::Model, // This is exclusively used to force the request to be authenticated
    access_token: crate::rs::guards::AccessToken,
    rdb: RedisConnection<RedisDb>
) -> Result<Json<ncryptf::Token>, Status> {
    let mut db = rdb.into_inner();
    let key: String = common::redis::create_redis_key(
        access_token.0.as_str(),
        common::redis::ACCESS_TOKEN_KEY_SUFFIX
    );
    let result: String = match db.get(key).await {
        Ok(result) => result,
        Err(_error) => {
            return Err(Status::InternalServerError);
        }
    };

    match serde_json::from_str::<common::auth::token::AccessToken>(&result) {
        Ok(token) => Ok(Json(token.token)),
        Err(_error) => {
            return Err(Status::InternalServerError);
        }
    }
}
