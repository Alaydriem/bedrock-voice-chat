#[allow(unused_imports)] // for rust-analyzer
use rocket_db_pools::deadpool_redis::redis::AsyncCommands;
use rocket_db_pools::deadpool_redis::Connection;
use serde::{Deserialize, Serialize};

use tracing::error;
const BVC_DEFAULT_ACCESS_TOKEN_LIFESPAN: i64 = 28800;
const BVC_DEFAULT_REFRESH_TOKEN_LIFESPAN: i64 = 2592000;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RefreshToken {
    pub user_id: i32,
    /// This is a cyclical reference so we can lookup the associated key
    pub access_token: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AccessToken {
    pub token: ncryptf::Token,
    pub user_id: i32,
}

impl AccessToken {
    /// Create's a new token for the user and stores it in Redis
    pub async fn new(
        user_id: i32,
        mut redis: Connection,
    ) -> Result<Self, ncryptf::rocket::TokenError> {
        let token = ncryptf::Token::new(BVC_DEFAULT_ACCESS_TOKEN_LIFESPAN);
        let at = Self { token, user_id };

        // Store the access token in redis
        let (): () = match redis
            .set_ex(
                crate::redis::create_redis_key(
                    at.token.access_token.as_str(),
                    crate::redis::ACCESS_TOKEN_KEY_SUFFIX,
                ),
                serde_json::to_string(&at).unwrap(),
                BVC_DEFAULT_ACCESS_TOKEN_LIFESPAN as usize,
            )
            .await
        {
            Ok(result) => result,
            Err(error) => {
                error!("Unable to connect to redis: {}", error.to_string());
                return Err(ncryptf::rocket::TokenError::ServerError);
            }
        };

        let rt = RefreshToken {
            user_id,
            access_token: at.clone().token.access_token,
        };

        // Store the refresh token in redis
        let (): () = match redis
            .set_ex(
                crate::redis::create_redis_key(
                    at.token.refresh_token.as_str(),
                    crate::redis::REFRESH_TOKEN_KEY_SUFFIX,
                ),
                serde_json::to_string(&rt).unwrap(),
                BVC_DEFAULT_REFRESH_TOKEN_LIFESPAN as usize,
            )
            .await
        {
            Ok(result) => result,
            Err(error) => {
                error!("Unable to connect to redis: {}", error.to_string());
                return Err(ncryptf::rocket::TokenError::ServerError);
            }
        };

        return Ok(at);
    }
}