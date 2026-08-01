use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

use super::ConnectCandidate;
use crate::structs::reachability::{AddressFamily, ServerReachability};

#[derive(Debug, Clone)]
pub struct CandidatePlan {
    candidates: Vec<ConnectCandidate>,
    v6_socket: bool,
}

impl CandidatePlan {
    // The budget a probe-endorsed family earns. Matches the per-port budget that
    // shipped before any family logic existed.
    pub const PREFERRED_BUDGET: Duration = Duration::from_secs(3);

    // The fallback family's budget. Shorter because reaching it means the preferred
    // family already failed, and the walk is bounded by the sum.
    pub const FALLBACK_BUDGET: Duration = Duration::from_millis(1500);

    // Ordered attempt list: port order is the operator's, family order is the
    // probe's verdict, and measured latency breaks ties inside a family. A negative
    // verdict changes order only — every candidate stays, so a wrong verdict costs
    // time and never connectivity.
    pub fn build(addrs: &[IpAddr], ports: &[u16], reachability: &ServerReachability) -> Self {
        let preference = reachability.preference();
        let v6_socket = addrs
            .iter()
            .any(|ip| AddressFamily::of(ip) == AddressFamily::Ipv6);

        let mut candidates = Vec::new();

        for port in ports {
            for family in preference.order() {
                let mut of_family: Vec<&IpAddr> = addrs
                    .iter()
                    .filter(|ip| AddressFamily::of(ip) == family)
                    .collect();

                // An address nothing measured sorts last rather than first, so a
                // silent endpoint never displaces one known to answer.
                of_family.sort_by_key(|ip| reachability.rtt_for(ip, *port).unwrap_or(u32::MAX));

                let budget = if preference.is_preferred(family) {
                    Self::PREFERRED_BUDGET
                } else {
                    Self::FALLBACK_BUDGET
                };

                for ip in of_family {
                    candidates.push(ConnectCandidate::new(
                        Self::dial_address(*ip, *port, v6_socket),
                        family,
                        *port,
                        budget,
                    ));
                }
            }
        }

        Self {
            candidates,
            v6_socket,
        }
    }

    pub fn requires_v6_socket(&self) -> bool {
        self.v6_socket
    }

    pub fn candidates(&self) -> &[ConnectCandidate] {
        &self.candidates
    }

    pub fn is_empty(&self) -> bool {
        self.candidates.is_empty()
    }

    // Used after a v6 bind fails: the v6 candidates have become undialable, and
    // what remains has to be unmapped because the socket is now plain IPv4.
    pub fn without_ipv6(&self) -> Self {
        let candidates = self
            .candidates
            .iter()
            .filter(|candidate| candidate.family() == AddressFamily::Ipv4)
            .map(|candidate| {
                ConnectCandidate::new(
                    Self::unmap(candidate.dial()),
                    AddressFamily::Ipv4,
                    candidate.port(),
                    candidate.budget(),
                )
            })
            .collect();

        Self {
            candidates,
            v6_socket: false,
        }
    }

    // s2n-quic writes a bare sockaddr_in for an IPv4 destination and does no
    // conversion of its own, so an IPv6 socket has to be handed the v4-mapped form
    // or the send fails with an address-family error.
    fn dial_address(ip: IpAddr, port: u16, v6_socket: bool) -> SocketAddr {
        match (ip, v6_socket) {
            (IpAddr::V4(v4), true) => SocketAddr::new(IpAddr::V6(v4.to_ipv6_mapped()), port),
            _ => SocketAddr::new(ip, port),
        }
    }

    fn unmap(addr: SocketAddr) -> SocketAddr {
        match addr.ip() {
            IpAddr::V6(v6) => match v6.to_ipv4_mapped() {
                Some(v4) => SocketAddr::new(IpAddr::V4(v4), addr.port()),
                None => addr,
            },
            IpAddr::V4(_) => addr,
        }
    }
}
