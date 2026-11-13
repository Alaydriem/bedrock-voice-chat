use std::time::Duration;

use common::structs::network::ServerEndpoint;

/// A resolved endpoint with how long it may be reused.
#[derive(Clone, Debug)]
pub struct CachedEndpoint {
    pub endpoint: ServerEndpoint,
    pub lifetime: Duration,
}
