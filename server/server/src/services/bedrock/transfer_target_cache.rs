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
        self.cache.insert(
            xuid.to_string(),
            TransferTarget { host, port },
        ).await;
    }

    pub async fn get(&self, xuid: &str) -> Option<TransferTarget> {
        self.cache.get(xuid).await
    }

    pub async fn remove(&self, xuid: &str) {
        self.cache.remove(xuid).await;
    }
}

#[cfg(test)]
mod tests {
    use super::TransferTargetCache;

    #[tokio::test]
    async fn test_transfer_target_cache_set_and_get() {
        let cache = TransferTargetCache::new(900);
        cache.set("2535428504476914", "192.168.1.100".to_string(), 19137).await;

        let target = cache.get("2535428504476914").await;
        assert!(target.is_some());
        let target = target.unwrap();
        assert_eq!(target.host, "192.168.1.100");
        assert_eq!(target.port, 19137);
    }

    #[tokio::test]
    async fn test_transfer_target_cache_missing() {
        let cache = TransferTargetCache::new(900);
        let target = cache.get("0000000000000000").await;
        assert!(target.is_none());
    }

    #[tokio::test]
    async fn test_transfer_target_cache_remove() {
        let cache = TransferTargetCache::new(900);
        cache.set("2535428504476914", "192.168.1.100".to_string(), 19137).await;
        cache.remove("2535428504476914").await;

        let target = cache.get("2535428504476914").await;
        assert!(target.is_none());
    }
}
