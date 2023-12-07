use common::{
    ncryptflib as ncryptf,
    pool::redis::RedisDb,
    rocket::http::Status,
    rocket_db_pools::Connection as RedisConnection,
};
use entity::user;
use ncryptf::rocket::Json;

#[allow(unused_imports)] // for rust-analyzer
use common::rocket_db_pools::deadpool_redis::redis::AsyncCommands;

/// Revokes the currently active refresh token
#[post("/token/revoke")]
pub async fn token_revoke(
    _user: user::Model, // This is exclusively used to force the request to be authenticated
    access_token: crate::rs::guards::AccessToken,
    rdb: RedisConnection<RedisDb>,
) -> Result<Json<bool>, Status> {
    let mut db = rdb.into_inner();
    let key: String = common::redis::create_redis_key(
        access_token.0.as_str(),
        common::redis::ACCESS_TOKEN_KEY_SUFFIX,
    );
    let result: String = match db.get(key.clone()).await {
        Ok(result) => result,
        Err(_error) => return Err(Status::InternalServerError),
    };

    match serde_json::from_str::<common::auth::token::AccessToken>(&result) {
        Ok(token) => {
            // Expire the access_token, which'll force Redis to delete it.
            let k: String = common::redis::create_redis_key(
                token.token.refresh_token.as_str(),
                common::redis::REFRESH_TOKEN_KEY_SUFFIX,
            );
            match db.expire(k, 0).await {
                Ok(r) => r,
                Err(_) => return Err(Status::InternalServerError),
            };

            // Expire the access_token, which'll force Redis to delete it.
            let _: bool = match db.expire(key, 0).await {
                Ok(r) => r,
                Err(_) => return Err(Status::InternalServerError),
            };

            return Ok(Json(true));
        }
        Err(_error) => return Err(Status::InternalServerError),
    }
}
