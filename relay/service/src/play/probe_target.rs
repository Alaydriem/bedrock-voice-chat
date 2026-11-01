use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

// Which addresses the registry will connect to on a node's word.
//
// A check-in names an address and a port, and the registry dials them. Anything that is
// not a public unicast address would let a node aim that connection at the registry's own
// network or at someone else's.
pub struct ProbeTarget;

impl ProbeTarget {
    pub fn is_global(address: IpAddr) -> bool {
        match address.to_canonical() {
            IpAddr::V4(v4) => Self::v4_is_global(v4),
            IpAddr::V6(v6) => Self::v6_is_global(v6),
        }
    }

    fn v4_is_global(a: Ipv4Addr) -> bool {
        let o = a.octets();

        !(a.is_unspecified()
            || a.is_loopback()
            || a.is_private()
            || a.is_link_local()
            || a.is_broadcast()
            || a.is_multicast()
            || a.is_documentation()
            // 100.64.0.0/10, carrier-grade NAT.
            || (o[0] == 100 && (o[1] & 0xc0) == 0x40)
            // 0.0.0.0/8 and 192.0.0.0/24, protocol assignments.
            || o[0] == 0
            || (o[0] == 192 && o[1] == 0 && o[2] == 0)
            // 198.18.0.0/15, benchmarking.
            || (o[0] == 198 && (o[1] & 0xfe) == 18)
            // 240.0.0.0/4, reserved.
            || o[0] >= 240)
    }

    fn v6_is_global(a: Ipv6Addr) -> bool {
        let s = a.segments();

        !(a.is_unspecified()
            || a.is_loopback()
            || a.is_multicast()
            // fc00::/7, unique local.
            || (s[0] & 0xfe00) == 0xfc00
            // fe80::/10, link-local.
            || (s[0] & 0xffc0) == 0xfe80
            // 2001:db8::/32, documentation.
            || (s[0] == 0x2001 && s[1] == 0x0db8))
    }
}
