mod plan;

pub use plan::CandidatePlan;

use std::net::SocketAddr;
use std::time::Duration;

use crate::structs::reachability::AddressFamily;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConnectCandidate {
    dial: SocketAddr,
    family: AddressFamily,
    port: u16,
    budget: Duration,
}

impl ConnectCandidate {
    pub fn new(dial: SocketAddr, family: AddressFamily, port: u16, budget: Duration) -> Self {
        Self {
            dial,
            family,
            port,
            budget,
        }
    }

    // The address to hand the QUIC client, which is the v4-mapped form when the
    // local socket is IPv6 rather than the address as resolved.
    pub fn dial(&self) -> SocketAddr {
        self.dial
    }

    pub fn family(&self) -> AddressFamily {
        self.family
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn budget(&self) -> Duration {
        self.budget
    }
}
