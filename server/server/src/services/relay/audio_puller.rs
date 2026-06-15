use anyhow::Error;

use super::client::RelayClient;

// Fetches a discovered `.opus` from a responding peer's audio endpoint into
// memory. Abstracted so the playback service can be exercised with a stub that
// returns canned bytes (or stalls, for cancellation tests) instead of a live
// HTTPS pull.
#[async_trait::async_trait]
pub trait AudioPuller: Send + Sync {
    async fn pull(&self, host: &str, http_port: u16, token: &str) -> Result<Vec<u8>, Error>;
}

// Production puller: a single-use HTTPS GET against the responder's audio
// endpoint via `RelayClient::pull_audio`.
pub struct RelayAudioPuller;

impl RelayAudioPuller {
    pub fn new() -> Self {
        Self
    }

    pub fn new_shared() -> std::sync::Arc<Self> {
        std::sync::Arc::new(Self::new())
    }
}

impl Default for RelayAudioPuller {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl AudioPuller for RelayAudioPuller {
    async fn pull(&self, host: &str, http_port: u16, token: &str) -> Result<Vec<u8>, Error> {
        RelayClient::pull_audio(host, http_port, token).await
    }
}
