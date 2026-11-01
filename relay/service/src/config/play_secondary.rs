use std::net::{IpAddr, SocketAddr};

use serde::{Deserialize, Serialize};

// One secondary nameserver holding a copy of the play zone.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaySecondaryConfig {
    // Addresses allowed to pull the zone by AXFR.
    pub transfer_from: Vec<IpAddr>,
    // Where the registry sends NOTIFY.
    pub notify: SocketAddr,
}
