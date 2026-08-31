use std::net::SocketAddr;
use std::time::Duration;

use iroh::endpoint::presets::Minimal;
use iroh::{Endpoint, EndpointAddr, PublicKey};

use crate::node::{NodeIdentity, PeerTicket, PeerTicketError};

use super::error::PeerError;

// A node's Iroh endpoint.
//
// Built on the `Minimal` preset, which sets the crypto provider and nothing
// else — no relay, no address lookup, no pkarr publishing. iroh has no ambient
// defaults to opt out of; `N0` is what reaches n0's infrastructure, and it is
// never chosen here.
//
// A peer is reachable at an address the far side already holds, which a peer ticket
// carries. There is no relay: this project never proxies traffic, so two peers that
// cannot reach each other directly do not connect at all.
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

    pub async fn bind(identity: &NodeIdentity) -> Result<Self, PeerError> {
        Self::bind_on(identity, None).await
    }

    /// Binds this endpoint, optionally on a port the operator chose.
    ///
    /// A ticket carries the addresses a peer will be dialled on, so the port is part
    /// of what an operator pastes into the far side's config. Left to the operating
    /// system it is a different port on every start, and the pasted value stops
    /// resolving to anywhere the moment this process restarts — which is invisible
    /// until someone tries to speak.
    pub async fn bind_on(identity: &NodeIdentity, port: Option<u16>) -> Result<Self, PeerError> {
        Self::bind_with_alpns(identity, port, vec![Self::ALPN.to_vec()]).await
    }

    /// Binds this endpoint answering on the ALPNs given rather than on the peer
    /// wire's alone.
    ///
    /// A parameter rather than a constant because one endpoint can serve more than
    /// one protocol: the registry answers enrollment and address observation on the
    /// same socket, and they authorize differently.
    ///
    /// No relay is configured, and there is no parameter through which one could be.
    /// A relay is a path this project's traffic must never take, and the only way to
    /// guarantee that is for the configuration not to exist.
    pub async fn bind_with_alpns(
        identity: &NodeIdentity,
        port: Option<u16>,
        alpns: Vec<Vec<u8>>,
    ) -> Result<Self, PeerError> {
        let mut builder = Endpoint::builder(Minimal)
            .secret_key(identity.secret_key().clone())
            .alpns(alpns);

        if let Some(port) = port {
            // The builder arrives pre-configured with an unspecified-address IPv4
            // socket, and `bind_addr` adds rather than replaces. Clearing first is
            // what makes this the only socket, and therefore the only port a ticket
            // can advertise.
            builder = builder
                .clear_ip_transports()
                .bind_addr(std::net::SocketAddr::from((
                    std::net::Ipv4Addr::UNSPECIFIED,
                    port,
                )))
                .map_err(|e| PeerError::Bind(e.to_string()))?;
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
    // Carries every address this endpoint can be dialled at, because a ticket is the
    // whole of what a peer is given: there is no relay to fall back to, so an address
    // missing from here is a path that does not exist. The loopback entry is what
    // keeps a bridge on this same host talking over `lo`.
    //
    // Iroh fills those addresses in after the bind returns, so a ticket minted
    // the instant a process starts would carry only the key. Waiting bounds
    // that: a ticket is minted rarely and by hand, and one that cannot be
    // dialled is worse than one that took a moment.
    pub async fn ticket(&self) -> Result<String, PeerTicketError> {
        self.ticket_advertising(None).await
    }

    /// A ticket that also names an address this endpoint was observed at.
    ///
    /// A node behind NAT reports only its LAN address, which no far side can dial.
    /// The observed address is added rather than substituted: the locally observed
    /// entries are what let a same-host bridge stay on `lo`, and iroh probes every
    /// candidate in parallel and keeps whichever answers.
    pub async fn ticket_advertising(
        &self,
        advertised: Option<SocketAddr>,
    ) -> Result<String, PeerTicketError> {
        let deadline = tokio::time::Instant::now() + Self::ADDRESS_WAIT;

        while tokio::time::Instant::now() < deadline {
            let addr = self.addr();
            if !addr.addrs.is_empty() {
                return PeerTicket::mint(&self.with_advertised(addr, advertised));
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        // Timed out with nothing to report. A key-only ticket is still valid for
        // a peer that reaches us some other way, so this is minted rather than
        // refused.
        PeerTicket::mint(&self.with_advertised(self.addr(), advertised))
    }

    fn with_advertised(
        &self,
        addr: EndpointAddr,
        advertised: Option<SocketAddr>,
    ) -> EndpointAddr {
        let mut addr = self.with_loopback(addr);

        if let Some(socket) = advertised {
            addr = addr.with_ip_addr(socket);
        }

        addr
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

    pub fn endpoint(&self) -> &Endpoint {
        &self.endpoint
    }

    pub async fn close(&self) {
        self.endpoint.close().await;
    }
}
