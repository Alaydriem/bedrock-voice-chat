use moka::future::Cache;
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone)]
pub struct AudioStreamTokenCache {
    // token → file_id
    cache: Arc<Cache<String, String>>,
}

impl AudioStreamTokenCache {
    pub fn new() -> Self {
        let cache = Cache::builder()
            .time_to_live(Duration::from_secs(60))
            .max_capacity(256)
            .build();

        Self {
            cache: Arc::new(cache),
        }
    }

    pub async fn create_token(&self, file_id: &str) -> String {
        let token = nanoid::nanoid!(32);
        self.cache.insert(token.clone(), file_id.to_string()).await;
        token
    }

    pub async fn validate_token(&self, token: &str) -> Option<String> {
        let file_id = self.cache.get(token).await?;
        self.cache.remove(token).await;
        Some(file_id)
    }
}

impl Default for AudioStreamTokenCache {
    fn default() -> Self {
        Self::new()
    }
}
