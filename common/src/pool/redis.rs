use crate::{
    rocket,
    rocket_db_pools::{self, deadpool_redis::Pool, Database},
};

#[derive(Database)]
#[database("cache")]
pub struct RedisDb(Pool);