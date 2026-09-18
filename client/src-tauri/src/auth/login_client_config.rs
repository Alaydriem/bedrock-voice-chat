use common::net::NetTimeouts;
use common::structs::reachability::AddressFamilyPreference;
use std::net::{IpAddr, Ipv4Addr};
use std::time::Duration;

/// How the login path builds its HTTP client.
///
/// Separate from the client it configures because a built `reqwest::Client` reports none of
/// this back. These are the settings whose absence produced a login that stopped waiting
/// before the server it was talking to could answer.
pub struct LoginClientConfig {
    timeout: Duration,
    connect_timeout: Duration,
    local_address: Option<IpAddr>,
}

impl LoginClientConfig {
    pub fn new(preference: AddressFamilyPreference) -> Self {
        Self {
            timeout: NetTimeouts::LOGIN,
            connect_timeout: NetTimeouts::CONNECT,
            local_address: match preference {
                AddressFamilyPreference::PreferIpv4 => Some(IpAddr::V4(Ipv4Addr::UNSPECIFIED)),
                AddressFamilyPreference::PreferIpv6 => None,
            },
        }
    }

    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    pub fn connect_timeout(&self) -> Duration {
        self.connect_timeout
    }

    pub fn local_address(&self) -> Option<IpAddr> {
        self.local_address
    }
}
