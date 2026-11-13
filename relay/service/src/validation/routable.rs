use std::net::IpAddr;

use common::structs::network::AddressClass;

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
    pub fn is_public(address: &str) -> bool {
        match address.parse::<IpAddr>() {
            Ok(ip) => AddressClass::of(ip).is_public(),
            // Not an address at all. Refused rather than probed: a hostname here is a
            // configuration the relay never asked for, and resolving it would let an
            // operator point the check at something other than what was published.
            Err(_) => false,
        }
    }
}
