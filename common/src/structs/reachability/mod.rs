mod certificate;
mod endpoint;
mod family;
mod outcome;
mod request;

pub use certificate::ObservedCertificate;
pub use endpoint::EndpointReachability;
pub use family::{AddressFamily, AddressFamilyPreference};
pub use outcome::{AnsweredVia, ReachabilityOutcome};
pub use request::ReachabilityRequest;

use serde::{Deserialize, Serialize};
use std::net::{IpAddr, SocketAddr};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct ServerReachability {
    host: String,
    quic: Vec<EndpointReachability>,
    https: Vec<EndpointReachability>,
    preference: AddressFamilyPreference,
}

impl ServerReachability {
    // The preference is derived here rather than supplied, so no caller can
    // publish a report whose verdict disagrees with its own measurements. Only
    // the QUIC endpoints count: HTTPS reports on a different transport, and the
    // family that carries voice is the one the verdict has to be right about.
    pub fn new(
        host: String,
        quic: Vec<EndpointReachability>,
        https: Vec<EndpointReachability>,
    ) -> Self {
        let preference = if quic
            .iter()
            .any(|e| e.family() == AddressFamily::Ipv6 && e.outcome().answered())
        {
            AddressFamilyPreference::PreferIpv6
        } else {
            AddressFamilyPreference::PreferIpv4
        };

        Self {
            host,
            quic,
            https,
            preference,
        }
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn quic(&self) -> &[EndpointReachability] {
        &self.quic
    }

    pub fn https(&self) -> &[EndpointReachability] {
        &self.https
    }

    pub fn preference(&self) -> AddressFamilyPreference {
        self.preference
    }

    pub fn rtt_for(&self, ip: &IpAddr, port: u16) -> Option<u32> {
        let wanted = SocketAddr::new(*ip, port).to_string();
        self.quic
            .iter()
            .find(|e| e.addr() == wanted)
            .and_then(|e| e.outcome().rtt_micros())
    }
}
