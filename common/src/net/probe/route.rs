use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, UdpSocket};

pub struct RouteProbe;

impl RouteProbe {
    // `connect` on a UDP socket installs a destination without emitting a packet.
    // The kernel performs route selection at that point, so an unroutable
    // destination fails here, and `local_addr` otherwise reports the source
    // address the stack would actually use.
    pub fn source_for(dest: SocketAddr) -> Option<IpAddr> {
        let bind: SocketAddr = match dest {
            SocketAddr::V4(_) => (Ipv4Addr::UNSPECIFIED, 0).into(),
            SocketAddr::V6(_) => (Ipv6Addr::UNSPECIFIED, 0).into(),
        };

        let socket = UdpSocket::bind(bind).ok()?;
        socket.connect(dest).ok()?;
        Some(socket.local_addr().ok()?.ip())
    }

    pub fn is_routable(dest: SocketAddr) -> bool {
        match Self::source_for(dest) {
            Some(source) => Self::source_suits(&source, &dest.ip()),
            None => false,
        }
    }

    // Whether a selected source can actually carry traffic to this destination.
    // Judging the source in isolation is not enough: a loopback source is correct
    // for a loopback destination, which is how an embedded server is reached, and
    // rejecting it would make a local deployment look unreachable.
    pub fn source_suits(source: &IpAddr, dest: &IpAddr) -> bool {
        if dest.is_loopback() {
            return source.is_loopback();
        }

        Self::is_global_unicast(source)
    }

    // Whether a selected source address represents connectivity to a public
    // server. Private and carrier-grade IPv4 qualify, because NAT is the normal
    // case; the IPv6 exclusions are the addresses that mean the host has no
    // native v6 path even though a v6 socket bound successfully.
    pub fn is_global_unicast(ip: &IpAddr) -> bool {
        match ip {
            IpAddr::V4(v4) => !v4.is_unspecified() && !v4.is_loopback() && !v4.is_link_local(),
            IpAddr::V6(v6) => {
                if v6.is_unspecified() || v6.is_loopback() {
                    return false;
                }

                if v6.to_ipv4_mapped().is_some() {
                    return false;
                }

                let leading = v6.segments()[0];
                let is_link_local = leading & 0xffc0 == 0xfe80;
                let is_unique_local = leading & 0xfe00 == 0xfc00;

                !is_link_local && !is_unique_local
            }
        }
    }
}
