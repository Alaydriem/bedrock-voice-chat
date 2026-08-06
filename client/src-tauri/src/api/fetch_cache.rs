use std::future::Future;
use std::time::Duration;

use moka::future::Cache;

/// One fetched value per key, held for a window.
///
/// Written for `/api/config`, where four callers read different fields of the same document
/// within a second of each other: the age gate, the spatial-audio and port refresh, the
/// bedrock connection hints, and the connect path's candidate planning. Each was its own
/// round trip on the screen the user is waiting on.
///
/// The TTL is the caller's to choose, because the right window depends on what the value
/// decides. `invalidate` covers what no TTL can: a connect that failed because the value
/// moved has to be able to re-ask immediately rather than wait out the window.
#[derive(Debug, Clone)]
pub struct FetchCache<V>
where
    V: Clone + Send + Sync + 'static,
{
    cache: Cache<String, V>,
}

impl<V> FetchCache<V>
where
    V: Clone + Send + Sync + 'static,
{
    pub fn new(ttl: Duration, capacity: u64) -> Self {
        Self {
            cache: Cache::builder()
                .time_to_live(ttl)
                .max_capacity(capacity)
                .build(),
        }
    }

    /// The stored value, or `fetch`'s result recorded under `key`.
    ///
    /// A failed fetch is deliberately not stored: caching an error would hand the same
    /// failure to every later caller in the window, and the retry that would have succeeded
    /// never happens.
    pub async fn get_or_fetch<F, Fut>(&self, key: &str, fetch: F) -> Result<V, String>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<V, String>>,
    {
        if let Some(value) = self.cache.get(key).await {
            return Ok(value);
        }

        let value = fetch().await?;
        self.cache.insert(key.to_string(), value.clone()).await;
        Ok(value)
    }

    pub async fn invalidate(&self, key: &str) {
        self.cache.invalidate(key).await;
    }
}
