use std::time::Duration;

use crate::structs::network::ServerEndpoint;

/// The result of resolving a server address.
#[derive(Clone, Debug)]
pub struct SrvResolution {
    pub endpoint: ServerEndpoint,
    // How long the answer may be reused. `None` for an explicit port or the default.
    pub ttl: Option<Duration>,
}
