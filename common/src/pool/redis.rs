use rocket;
use rocket_db_pools::{self, deadpool_redis::Pool, Database};

#[derive(Database)]
#[database("cache")]
pub struct RedisDb(Pool);

pub fn create_redis_key(key: &str, suffix: &str) -> String {
    let key = blake3::hash(format!("{}+{}", key, suffix).as_bytes());
    return key.to_hex().to_string();
}
