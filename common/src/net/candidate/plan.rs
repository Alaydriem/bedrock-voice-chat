use std::net::{IpAddr, SocketAddr};

use super::ConnectCandidate;
use crate::net::NetTimeouts;
use crate::structs::reachability::{AddressFamily, ServerReachability};

#[derive(Debug, Clone)]
pub struct CandidatePlan {
    candidates: Vec<ConnectCandidate>,
    v6_socket: bool,
}

impl CandidatePlan {
    // Ordered attempt list: ports the probe reached lead ports it did not, the operator's
    // order decides between ports of equal standing, family order is the probe's verdict,
    // and measured latency breaks ties inside a family. A negative verdict changes order
    // only — every candidate stays, so a wrong verdict costs time and never connectivity.
    //
    // A port that answered nothing used to lead the walk purely because it was advertised
    // first, and cost a full handshake budget before the walk reached a port that answers
    // instantly — with the report saying so already in hand.
    //
    // Ports that all answered keep the operator's order rather than racing on latency,
    // because that order is a routing instruction and not a preference. The advertised
    // port is how clients are steered through Meridian, and the backend's own port is
    // reachable and faster wherever it is exposed; ranking those two by latency would walk
    // every client around the proxy that provides their tenant routing.
    //
    // Every candidate gets the same budget. A shorter one for the fallback family bounded
    // the walk, but it also meant the attempt made after the preferred family had already
    // failed — the one most likely to be the unusual path that actually works — was the
    // attempt given the least time to complete.
    pub fn build(addrs: &[IpAddr], ports: &[u16], reachability: &ServerReachability) -> Self {
        let preference = reachability.preference();
        let v6_socket = addrs
            .iter()
            .any(|ip| AddressFamily::of(ip) == AddressFamily::Ipv6);

        // Stable, so the operator's order decides within each group: among the ports that
        // answered, and among the ports that did not.
        let mut ports = ports.to_vec();
        ports.sort_by_key(|port| !Self::answered_on(addrs, *port, reachability));

        let mut candidates = Vec::new();

        for port in &ports {
            for family in preference.order() {
                let mut of_family: Vec<&IpAddr> = addrs
                    .iter()
                    .filter(|ip| AddressFamily::of(ip) == family)
                    .collect();

                // An address nothing measured sorts last rather than first, so a
                // silent endpoint never displaces one known to answer.
                of_family.sort_by_key(|ip| reachability.rtt_for(ip, *port).unwrap_or(u32::MAX));

                for ip in of_family {
                    candidates.push(ConnectCandidate::new(
                        Self::dial_address(*ip, *port, v6_socket),
                        family,
                        *port,
                        NetTimeouts::HANDSHAKE,
                    ));
                }
            }
        }

        Self {
            candidates,
            v6_socket,
        }
    }

    // One address answering is the whole of what this asks. Requiring more would let a
    // single dead address sink a port that another address reaches instantly.
    fn answered_on(addrs: &[IpAddr], port: u16, reachability: &ServerReachability) -> bool {
        addrs
            .iter()
            .any(|ip| reachability.rtt_for(ip, port).is_some())
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
