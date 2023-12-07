use common::{
    ncryptflib as ncryptf,
    pool::redis::RedisDb,
    rocket::http::Status,
    rocket_db_pools::Connection as RedisConnection,
};
use ncryptf::rocket::Json;

#[allow(unused_imports)] // for rust-analyzer
use common::rocket_db_pools::deadpool_redis::redis::AsyncCommands;

/// Refreshes the current token and expires the old ones if they are still present
/// This endpoint does not require authentication
#[post("/token/refresh?<refresh_token>")]
pub async fn token_refresh(
    refresh_token: String,
    rdb: RedisConnection<RedisDb>,
) -> Result<Json<ncryptf::Token>, Status> {
    let mut db = rdb.into_inner();
    let mut key: String = common::redis::create_redis_key(
        refresh_token.as_str(),
        common::redis::REFRESH_TOKEN_KEY_SUFFIX,
    );

    let rt: common::auth::token::RefreshToken = match db.get::<String, String>(key.clone()).await {
        Ok(result) => match serde_json::from_str(result.as_str()) {
            Ok(result) => result,
            Err(_error) => return Err(Status::InternalServerError),
        },
        Err(_error) => return Err(Status::InternalServerError),
    };

    // Delete the refresh token
    match db.expire(key, 0).await {
        Ok(r) => r,
        Err(_) => return Err(Status::InternalServerError),
    };

    // Delete the access token
    key = common::redis::create_redis_key(
        rt.access_token.as_str(),
        common::redis::ACCESS_TOKEN_KEY_SUFFIX,
    );
    let _: bool = match db.expire(key, 0).await {
        Ok(r) => r,
        Err(_) => return Err(Status::InternalServerError),
    };

    // Generate a new access token and return the underlying token
    match common::auth::token::AccessToken::new(rt.user_id, db).await {
        Ok(access_token) => return Ok(Json(access_token.token)),
        Err(_) => return Err(Status::InternalServerError),
    }
}
