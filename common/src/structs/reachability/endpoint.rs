use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use ts_rs::TS;

use super::{AddressFamily, ObservedCertificate, ReachabilityOutcome};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct EndpointReachability {
    // The address exactly as dialed, so a v4-mapped attempt is distinguishable
    // from a native v4 one in a report.
    addr: String,
    family: AddressFamily,
    port: u16,
    outcome: ReachabilityOutcome,
    certificate: Option<ObservedCertificate>,
}

impl EndpointReachability {
    pub fn new(
        dialed: SocketAddr,
        outcome: ReachabilityOutcome,
        certificate: Option<ObservedCertificate>,
    ) -> Self {
        Self {
            addr: dialed.to_string(),
            family: AddressFamily::of(&dialed.ip()),
            port: dialed.port(),
            outcome,
            certificate,
        }
    }

    pub fn addr(&self) -> &str {
        &self.addr
    }

    pub fn family(&self) -> AddressFamily {
        self.family
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn outcome(&self) -> &ReachabilityOutcome {
        &self.outcome
    }

    pub fn certificate(&self) -> Option<&ObservedCertificate> {
        self.certificate.as_ref()
    }
}
