use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

// Whether a declared address is one the relay can reach from the internet.
//
// This decides whether the address half of a daily validation runs at all. A public
// address is probed, because an operator who declares one they do not control would
// otherwise have the relay fronting a third party's host from its own zone. A private
// one is published and never probed: it is unreachable from here by construction, and
// it cannot front anyone either — it resolves only on the network of whoever declared
// it.
pub struct RoutableAddress;

impl RoutableAddress {
    // The CGNAT range. Not covered by `is_private`, and an operator behind a carrier
    // NAT has no other address to give.
    const CGNAT: Ipv4Addr = Ipv4Addr::new(100, 64, 0, 0);
    const CGNAT_PREFIX_BITS: u32 = 10;

    pub fn is_public(address: &str) -> bool {
        match address.parse::<IpAddr>() {
            Ok(IpAddr::V4(v4)) => Self::is_public_v4(v4),
            Ok(IpAddr::V6(v6)) => Self::is_public_v6(v6),
            // Not an address at all. Refused rather than probed: a hostname here is a
            // configuration the relay never asked for, and resolving it would let an
            // operator point the check at something other than what was published.
            Err(_) => false,
        }
    }

    fn is_public_v4(address: Ipv4Addr) -> bool {
        if address.is_private()
            || address.is_loopback()
            || address.is_link_local()
            || address.is_broadcast()
            || address.is_documentation()
            || address.is_unspecified()
            || address.is_multicast()
        {
            return false;
        }

        !Self::in_cgnat(address)
    }

    fn in_cgnat(address: Ipv4Addr) -> bool {
        let bits = u32::from(address);
        let base = u32::from(Self::CGNAT);
        let mask = u32::MAX << (32 - Self::CGNAT_PREFIX_BITS);
        bits & mask == base & mask
    }

    fn is_public_v6(address: Ipv6Addr) -> bool {
        if address.is_loopback() || address.is_unspecified() || address.is_multicast() {
            return false;
        }

        // Unique local (fc00::/7) and link-local (fe80::/10). Neither has a stable
        // predicate on stable Rust, so the prefixes are checked directly.
        let first = address.segments()[0];
        let unique_local = first & 0xfe00 == 0xfc00;
        let link_local = first & 0xffc0 == 0xfe80;

        !unique_local && !link_local
    }
}
