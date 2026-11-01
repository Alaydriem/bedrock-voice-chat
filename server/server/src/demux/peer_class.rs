use std::net::IpAddr;

/// Classifies the peer of an accepted connection.
pub struct PeerClass;

impl PeerClass {
    /// True for `127.0.0.0/8`, `::1` and `::ffff:127.0.0.0/104`.
    ///
    /// The public socket is dual-stack, so a local IPv4 client arrives as an IPv4-mapped
    /// IPv6 address. Reading only `is_loopback` on the raw address would refuse it.
    pub fn is_loopback(ip: IpAddr) -> bool {
        match ip {
            IpAddr::V4(v4) => v4.is_loopback(),
            IpAddr::V6(v6) => {
                v6.is_loopback() || v6.to_ipv4_mapped().is_some_and(|v4| v4.is_loopback())
            }
        }
    }
}
