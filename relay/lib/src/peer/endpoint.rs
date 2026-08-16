use std::time::Duration;

use iroh::endpoint::presets::Minimal;
use iroh::{Endpoint, EndpointAddr, PublicKey, RelayConfig, RelayMap, RelayMode, RelayUrl};
use iroh_relay::RelayQuicConfig;

use crate::node::{NodeIdentity, PeerTicket, PeerTicketError};

use super::error::PeerError;

// A node's Iroh endpoint.
//
// Built on the `Minimal` preset, which sets the crypto provider and nothing
// else — no relay, no address lookup, no pkarr publishing. iroh has no ambient
// defaults to opt out of; `N0` is what reaches n0's infrastructure, and it is
// never chosen here.
//
// Without a relay a peer is reachable only at an address the far side already
// holds, which is a local-network or same-host arrangement. The relay is what
// makes a peer ticket enough on its own, so it is supplied rather than assumed.
pub struct PeerEndpoint {
    endpoint: Endpoint,
    node_id: PublicKey,
}

impl PeerEndpoint {
    // Version-bearing so a future wire revision can be negotiated by ALPN before
    // a byte of the control stream is read.
    pub const ALPN: &'static [u8] = b"bvc-peer/1";

    // Long enough for a local bind to report its interfaces, short enough that a
    // command an operator is waiting on still feels immediate.
    const ADDRESS_WAIT: Duration = Duration::from_secs(2);

    // Environment variable an operator running their own relay overrides the
    // built-in token with. They cannot recompile, so without this the token
    // would make a self-hosted relay unusable rather than merely restricted.
    const TOKEN_ENV: &'static str = "BVC_RELAY_ACCESS_TOKEN";

    pub async fn bind(
        identity: &NodeIdentity,
        relay_url: Option<RelayUrl>,
    ) -> Result<Self, PeerError> {
        let mut builder = Endpoint::builder(Minimal)
            .secret_key(identity.secret_key().clone())
            .alpns(vec![Self::ALPN.to_vec()]);

        if let Some(url) = relay_url {
            // `RelayConfig` is non-exhaustive, so it is built through its
            // constructor and the token set after. The QUIC config is supplied
            // rather than left off: without address discovery a peer never
            // learns its own public address, and every pair relays forever
            // instead of upgrading to a direct path.
            let mut relay = RelayConfig::new(url, Some(RelayQuicConfig::default()));
            relay.auth_token = Self::access_token();

            builder = builder.relay_mode(RelayMode::Custom(RelayMap::from_iter([relay])));
        }

        let endpoint = builder
            .bind()
            .await
            .map_err(|e| PeerError::Bind(e.to_string()))?;

        Ok(Self {
            node_id: identity.node_id(),
            endpoint,
        })
    }

    pub fn node_id(&self) -> PublicKey {
        self.node_id
    }

    // What this endpoint currently believes it is reachable at.
    //
    // Iroh fills the direct addresses and the home relay in after the bind
    // returns, so this is a snapshot rather than a constant: read it when a
    // ticket is wanted, not once at startup.
    pub fn addr(&self) -> EndpointAddr {
        self.endpoint.addr()
    }

    // The value an operator gives the far side.
    //
    // Carries the direct addresses as well as the relay, which is what lets one
    // ticket serve a peer across the internet and a bridge on this same host:
    // iroh prefers a direct path and falls back to the relay, so a loopback
    // address in the ticket is what keeps same-host traffic on `lo`.
    //
    // Iroh fills those addresses in after the bind returns, so a ticket minted
    // the instant a process starts would carry only the key. Waiting bounds
    // that: a ticket is minted rarely and by hand, and one that cannot be
    // dialled is worse than one that took a moment.
    pub async fn ticket(&self) -> Result<String, PeerTicketError> {
        let deadline = tokio::time::Instant::now() + Self::ADDRESS_WAIT;

        while tokio::time::Instant::now() < deadline {
            let addr = self.addr();
            if !addr.addrs.is_empty() {
                return PeerTicket::mint(&self.with_loopback(addr));
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        // Timed out with nothing to report. A key-only ticket is still valid for
        // a peer that reaches us some other way, so this is minted rather than
        // refused.
        PeerTicket::mint(&self.with_loopback(self.addr()))
    }

    // Adds this endpoint's loopback address to what iroh reports.
    //
    // Iroh omits loopback from its own address set, which is right for a ticket
    // crossing the internet and wrong for the deployment where a bridge shares a
    // host with the server. Such a bridge would otherwise have to reach us at a
    // LAN address, which works until the host has no interface up — an isolated
    // container has exactly one usable path to us, and it is this one.
    //
    // A remote peer that tries it simply fails that candidate; iroh probes paths
    // in parallel and keeps whichever answers.
    fn with_loopback(&self, addr: EndpointAddr) -> EndpointAddr {
        let mut addr = addr;

        for socket in self.endpoint.bound_sockets() {
            addr = addr.with_ip_addr(std::net::SocketAddr::new(
                std::net::Ipv4Addr::LOCALHOST.into(),
                socket.port(),
            ));
        }

        addr
    }

    // The token presented to our relay, baked in at build time.
    //
    // Not a secret: it ships inside every binary and anyone who looks can read
    // it. It raises the cost of using our relay from "point at the URL" to
    // "extract a string from a binary", and nothing more is claimed for it. What
    // protects a call is the peer's key and its grant, neither of which is here.
    //
    // `None` on a build with the variable unset, which is every local build —
    // and a relay running `access = "everyone"` takes those.
    fn access_token() -> Option<String> {
        std::env::var(Self::TOKEN_ENV)
            .ok()
            .filter(|token| !token.is_empty())
            .or_else(|| option_env!("BVC_RELAY_ACCESS_TOKEN").map(str::to_string))
            .filter(|token| !token.is_empty())
    }

    pub fn endpoint(&self) -> &Endpoint {
        &self.endpoint
    }

    pub async fn close(&self) {
        self.endpoint.close().await;
    }
}
