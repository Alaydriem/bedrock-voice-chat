use moka::future::Cache;
use std::time::Duration;

use super::TransferTarget;

#[derive(Clone)]
pub struct TransferTargetCache {
    cache: Cache<String, TransferTarget>,
}

impl TransferTargetCache {
    pub fn new(ttl_secs: u64) -> Self {
        let cache = Cache::builder()
            .time_to_live(Duration::from_secs(ttl_secs))
            .max_capacity(256)
            .build();
        Self { cache }
    }

    pub async fn set(&self, xuid: &str, host: String, port: u16) {
        self.cache
            .insert(xuid.to_string(), TransferTarget { host, port })
            .await;
    }

    pub async fn get(&self, xuid: &str) -> Option<TransferTarget> {
        self.cache.get(xuid).await
    }

    pub async fn remove(&self, xuid: &str) {
        self.cache.remove(xuid).await;
    }
}
