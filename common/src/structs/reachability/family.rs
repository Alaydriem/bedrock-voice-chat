use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub enum AddressFamily {
    Ipv4,
    Ipv6,
}

impl AddressFamily {
    // A v4-mapped address is an IPv4 destination wearing a v6 sockaddr, so it is
    // classified by what it reaches rather than by how it is spelled.
    pub fn of(ip: &IpAddr) -> Self {
        match ip {
            IpAddr::V4(_) => Self::Ipv4,
            IpAddr::V6(v6) => match v6.to_ipv4_mapped() {
                Some(_) => Self::Ipv4,
                None => Self::Ipv6,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub enum AddressFamilyPreference {
    PreferIpv6,
    PreferIpv4,
}

impl Default for AddressFamilyPreference {
    // IPv4 is the default because it is what every released client does today. A
    // host only moves off it once a probe has seen IPv6 answering.
    fn default() -> Self {
        Self::PreferIpv4
    }
}

impl AddressFamilyPreference {
    pub fn order(&self) -> [AddressFamily; 2] {
        match self {
            Self::PreferIpv6 => [AddressFamily::Ipv6, AddressFamily::Ipv4],
            Self::PreferIpv4 => [AddressFamily::Ipv4, AddressFamily::Ipv6],
        }
    }

    pub fn is_preferred(&self, family: AddressFamily) -> bool {
        self.order()[0] == family
    }
}
