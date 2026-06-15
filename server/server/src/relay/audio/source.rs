use std::sync::Arc;

use common::structs::relay::{AudioAvailable, AudioQuery};

use crate::services::AudioStreamTokenCache;

use super::file_existence::AudioFileExistence;

// Responder half of the cross-server jukebox peer-link discovery handshake.
// On an inbound `AudioQuery`, if this server holds the file it mints a stream
// token and answers with `AudioAvailable`; otherwise it stays silent.
pub struct AudioSource {
    token_cache: AudioStreamTokenCache,
    existence: Arc<dyn AudioFileExistence>,
}

impl AudioSource {
    pub fn new(token_cache: AudioStreamTokenCache, existence: Arc<dyn AudioFileExistence>) -> Self {
        Self {
            token_cache,
            existence,
        }
    }

    pub fn new_shared(
        token_cache: AudioStreamTokenCache,
        existence: Arc<dyn AudioFileExistence>,
    ) -> Arc<Self> {
        Arc::new(Self::new(token_cache, existence))
    }

    // Mints a fresh stream token for `audio_id` and produces the `AudioAvailable`
    // to send back over the peer link, or `None` when the file is not held here.
    // The minted token is 60s TTL + single-use (see `audio_stream_token_cache`):
    // the fulfiller HTTP-pulls the `.opus` with it exactly once before it expires.
    pub async fn handle_query(&self, query: &AudioQuery) -> Option<AudioAvailable> {
        if !self.existence.has_audio(&query.audio_id).await {
            return None;
        }
        let stream_token = self.token_cache.create_token(&query.audio_id).await;
        Some(AudioAvailable {
            audio_id: query.audio_id.clone(),
            stream_token,
            correlation_id: query.correlation_id.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct StubExistence {
        present: bool,
    }

    #[async_trait::async_trait]
    impl AudioFileExistence for StubExistence {
        async fn has_audio(&self, _audio_id: &str) -> bool {
            self.present
        }
    }

    fn source_with(present: bool) -> AudioSource {
        AudioSource::new(
            AudioStreamTokenCache::new(),
            Arc::new(StubExistence { present }),
        )
    }

    // A server that HAS the file answers with an `AudioAvailable` whose token
    // validates back to the queried `audio_id` via the token cache.
    #[tokio::test]
    async fn answers_with_validatable_token_when_file_present() {
        let cache = AudioStreamTokenCache::new();
        let source = AudioSource::new(cache.clone(), Arc::new(StubExistence { present: true }));
        let query = AudioQuery {
            audio_id: "audio-present".into(),
            correlation_id: "corr-1".into(),
        };

        let available = source.handle_query(&query).await.expect("should answer");
        assert_eq!(available.audio_id, "audio-present");
        assert_eq!(available.correlation_id, "corr-1");

        let resolved = cache.validate_token(&available.stream_token).await;
        assert_eq!(resolved, Some("audio-present".to_string()));
    }

    // A server WITHOUT the file stays silent.
    #[tokio::test]
    async fn no_answer_when_file_absent() {
        let source = source_with(false);
        let query = AudioQuery {
            audio_id: "audio-missing".into(),
            correlation_id: "corr-1".into(),
        };
        assert!(source.handle_query(&query).await.is_none());
    }
}
