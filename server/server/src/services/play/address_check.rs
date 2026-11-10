use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};

use common::structs::network::AddressClass;

use super::PlayAddressProblem;

/// Refuses, before the registry is asked anything, an address that cannot work.
pub struct PlayAddressCheck;

impl PlayAddressCheck {
    // A documentation address (RFC 5737). Connecting a UDP socket to it sends nothing and
    // makes the operating system pick the interface the default route would use.
    const ROUTE_PROBE: SocketAddr =
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 0, 2, 1)), 9);

    /// A configured address must be public. The interface plays no part: an operator who
    /// states the address knows the network better than the interface does. Only this
    /// problem stops play mode; see [`Self::advisory`] for the one that does not.
    pub fn check(configured: Option<IpAddr>) -> Result<(), PlayAddressProblem> {
        match configured {
            Some(address) => {
                let class = AddressClass::of(address);
                if class.is_public() {
                    Ok(())
                } else {
                    Err(PlayAddressProblem::ConfiguredNotPublic { address, class })
                }
            }
            None => Ok(()),
        }
    }

    /// A problem the registry's preflight is left to confirm. With no address configured, a
    /// default route in 100.64.0.0/10 may be carrier-grade NAT, or may be a hosted pod
    /// network or a VPN whose port is forwarded to the internet, so the answer is a warning
    /// and not a stop.
    pub fn advisory(
        configured: Option<IpAddr>,
        default_route: Option<Ipv4Addr>,
    ) -> Option<PlayAddressProblem> {
        if configured.is_some() {
            return None;
        }

        match default_route {
            Some(interface)
                if AddressClass::of(IpAddr::V4(interface)) == AddressClass::Cgnat =>
            {
                Some(PlayAddressProblem::BehindCgnat { interface })
            }
            _ => None,
        }
    }

    /// The IPv4 address of the interface the default route leaves through. `None` when
    /// there is no route or the address is not IPv4.
    pub fn default_route_ipv4() -> Option<Ipv4Addr> {
        let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).ok()?;
        socket.connect(Self::ROUTE_PROBE).ok()?;
        match socket.local_addr().ok()?.ip() {
            IpAddr::V4(address) => Some(address),
            IpAddr::V6(_) => None,
        }
    }
}
