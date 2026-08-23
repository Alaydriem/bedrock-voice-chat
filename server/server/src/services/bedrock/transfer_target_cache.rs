use moka::future::Cache;
use std::time::Duration;

use super::TransferTarget;

#[derive(Clone)]
/// Where a player's BVC Connect handoff should send them.
///
/// Keyed on the gamertag, not the xuid. The xuid is only present when registration supplied
/// one and the Realms path never returns it, so a large share of players have no xuid the
/// server knows — while the gamertag is on both sides: the caller's certificate CN, and
/// `conn.player().name` off the verified Bedrock login chain.
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

    pub async fn set(&self, gamertag: &str, host: String, port: u16) {
        self.cache
            .insert(gamertag.to_string(), TransferTarget { host, port })
            .await;
    }

    pub async fn get(&self, gamertag: &str) -> Option<TransferTarget> {
        self.cache.get(gamertag).await
    }

    pub async fn remove(&self, gamertag: &str) {
        self.cache.remove(gamertag).await;
    }
}
