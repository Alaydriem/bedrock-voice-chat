use std::net::SocketAddr;

// What this server puts in its peer ticket as a dialable address.
//
// The registry supplies the IP and nothing else. The port it observed is the source
// port of this server's outbound probe, not the port a far side would dial: a NAT
// with endpoint-independent mapping happens to preserve the pinned one, and a NAT
// that rewrites does not. The configured `peer_port` is the port the operator
// actually forwarded, so that is the one advertised.
pub struct AdvertisedAddress;

impl AdvertisedAddress {
    // `None` unless both halves are known. Without a pinned port there is nothing
    // forwarded, and without an observation there is no address this server can vouch
    // for — advertising either alone promises reachability it does not have.
    pub fn from_observation(
        observed: Option<SocketAddr>,
        peer_port: Option<u16>,
    ) -> Option<SocketAddr> {
        Some(SocketAddr::new(observed?.ip(), peer_port?))
    }
}
