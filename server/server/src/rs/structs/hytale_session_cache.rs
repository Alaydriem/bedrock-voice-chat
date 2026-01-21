use common::auth::DeviceFlow;
use moka::future::Cache;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// A Hytale device flow session stored in the cache
#[derive(Clone, Debug)]
pub struct HytaleSession {
    /// The device flow data from the auth provider
    pub flow: DeviceFlow,
    /// When this session expires
    pub expires_at: Instant,
}

/// Cache manager for Hytale device flow sessions
#[derive(Clone)]
pub struct HytaleSessionCache {
    cache: Arc<Cache<String, HytaleSession>>,
}

impl HytaleSessionCache {
    /// Create a new session cache with 15 minute TTL
    pub fn new() -> Self {
        let cache = Cache::builder()
            .time_to_live(Duration::from_secs(15 * 60)) // 15 minutes
            .max_capacity(10000)
            .build();

        Self {
            cache: Arc::new(cache),
        }
    }

    /// Insert a new session into the cache
    pub async fn insert(&self, session_id: String, session: HytaleSession) {
        self.cache.insert(session_id, session).await;
    }

    /// Get a session from the cache
    pub async fn get(&self, session_id: &str) -> Option<HytaleSession> {
        self.cache.get(session_id).await
    }

    /// Remove a session from the cache
    pub async fn remove(&self, session_id: &str) {
        self.cache.remove(session_id).await;
    }

    /// Check if a session exists and is not expired
    pub async fn is_valid(&self, session_id: &str) -> bool {
        if let Some(session) = self.cache.get(session_id).await {
            return session.expires_at > Instant::now();
        }
        false
    }
}

impl Default for HytaleSessionCache {
    fn default() -> Self {
        Self::new()
    }
}
