use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};

use common::bedrock_protocol::{IceScope, ProxyConfig};

/// Which local interfaces the Bedrock proxy listens on.
///
/// `Network` is what the app needs: Minecraft reaches the proxy over the LAN,
/// through LAN discovery, or at a typed address.
///
/// `Loopback` keeps every proxy socket on `127.0.0.1`: HTTP signaling, ICE, and
/// no LAN discovery. Windows Defender Firewall asks for permission each time a
/// new executable listens on a non-loopback address, and each rebuilt test
/// binary is a new executable. The e2e harness manages this value so that its
/// proxy never causes that prompt. A client that dials a `Loopback` proxy must
/// use `IceScope::LoopbackOnly` too, or ICE finds no candidate pair.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum BedrockListenScope {
    #[default]
    Network,
    Loopback,
}

impl BedrockListenScope {
    /// The HTTP signaling address for `port`.
    ///
    /// `Network` binds `[::]`, not `0.0.0.0`: the listener is dual-stack, so
    /// one socket answers both the IPv6 dial that follows LAN discovery and a
    /// typed `127.0.0.1:<port>`. An IPv4-only bind leaves the discovered entry
    /// unjoinable.
    pub fn listen_addr(self, port: u16) -> SocketAddr {
        match self {
            Self::Network => SocketAddr::from((Ipv6Addr::UNSPECIFIED, port)),
            Self::Loopback => SocketAddr::from((Ipv4Addr::LOCALHOST, port)),
        }
    }

    /// Applies the scope to the interface settings of `config`.
    ///
    /// `Network` keeps the library defaults for LAN discovery and ICE.
    pub fn restrict(self, config: ProxyConfig) -> ProxyConfig {
        match self {
            Self::Network => config,
            Self::Loopback => ProxyConfig {
                lan_discovery: None,
                ice_scope: IceScope::LoopbackOnly,
                ..config
            },
        }
    }
}
