use common::auth::DeviceFlow;
use std::time::Instant;

/// A Hytale device flow session stored in the cache
#[derive(Clone, Debug)]
pub struct HytaleSession {
    /// The device flow data from the auth provider
    pub flow: DeviceFlow,
    /// When this session expires
    pub expires_at: Instant,
}
