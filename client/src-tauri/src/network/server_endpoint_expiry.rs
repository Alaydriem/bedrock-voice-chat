use std::time::{Duration, Instant};

use moka::Expiry;

use super::CachedEndpoint;

/// Expires each cached endpoint after its own lifetime.
pub struct ServerEndpointExpiry;

impl ServerEndpointExpiry {
    pub const MIN_LIFETIME: Duration = Duration::from_secs(30);
    pub const MAX_LIFETIME: Duration = Duration::from_secs(300);
}

impl Expiry<String, CachedEndpoint> for ServerEndpointExpiry {
    fn expire_after_create(
        &self,
        _key: &String,
        value: &CachedEndpoint,
        _created_at: Instant,
    ) -> Option<Duration> {
        Some(value.lifetime)
    }

    fn expire_after_update(
        &self,
        _key: &String,
        value: &CachedEndpoint,
        _updated_at: Instant,
        _duration_until_expiry: Option<Duration>,
    ) -> Option<Duration> {
        Some(value.lifetime)
    }
}
