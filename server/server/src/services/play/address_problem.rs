use std::net::{IpAddr, Ipv4Addr};

use common::structs::network::AddressClass;

/// Why a play-mode server cannot be reached from the internet, known before any registry
/// request is made.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayAddressProblem {
    /// `server.tls.play.address` names an address players on the internet cannot reach.
    ConfiguredNotPublic { address: IpAddr, class: AddressClass },
    /// No address is configured and the default route leaves through 100.64.0.0/10. A
    /// warning: the preflight decides whether the server can be reached.
    BehindCgnat { interface: Ipv4Addr },
}
