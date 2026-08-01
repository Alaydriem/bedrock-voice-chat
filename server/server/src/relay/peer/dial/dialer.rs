use std::future::Future;
use std::net::{IpAddr, SocketAddr};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context as TaskContext, Poll};

use anyhow::Context;
use bytes::Bytes;
use common::s2n_quic::client::Connect;
use common::s2n_quic::{Client, Connection};
use common::structs::packet::QuicNetworkPacket;
use tokio::sync::mpsc;

use crate::relay::peer::link::ingest_sink::GatedPeerIngest;
use crate::relay::relayed_packet::RelayedPacket;

// Adapts s2n-quic's poll-based datagram receive into an awaitable future,
// matching the server's own input stream (`stream/quic/stream_manager/input.rs`).
struct RecvDatagram<'c> {
    conn: &'c Connection,
}

impl<'c> Future for RecvDatagram<'c> {
    type Output = Result<Bytes, anyhow::Error>;
    fn poll(self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<Self::Output> {
        match self.conn.datagram_mut(
            |r: &mut common::s2n_quic::provider::datagram::default::Receiver| {
                r.poll_recv_datagram(cx)
            },
        ) {
            Ok(Poll::Ready(Ok(bytes))) => Poll::Ready(Ok(bytes)),
            Ok(Poll::Ready(Err(e))) => Poll::Ready(Err(anyhow::anyhow!(e))),
            Ok(Poll::Pending) => Poll::Pending,
            Err(e) => Poll::Ready(Err(anyhow::anyhow!(e))),
        }
    }
}

// Establishes the initiator (QUIC client) side of a peer link and pumps packets
// in both directions over its single bidirectional QUIC connection.
//
// The credential is the in-memory client cert the ACCEPTOR issued via
// `CertificateService::sign_peer_cert` after a successful presence proof; the CA
// is the acceptor's CA. Builds a `Client`, connects with the peer's server name,
// then drives datagrams.
pub struct PeerDialer {
    ca_pem: Vec<u8>,
    cert_pem: Vec<u8>,
    key_pem: Vec<u8>,
}

impl PeerDialer {
    pub fn new(ca_pem: Vec<u8>, cert_pem: Vec<u8>, key_pem: Vec<u8>) -> Self {
        Self {
            ca_pem,
            cert_pem,
            key_pem,
        }
    }

    // A dual-stack socket is used only when a peer actually has an IPv6 address.
    // IPv4 destinations then travel v4-mapped, because s2n-quic hands the kernel a
    // bare sockaddr_in otherwise and an IPv6 socket rejects it.
    pub fn bind_address(addrs: &[IpAddr]) -> &'static str {
        let has_ipv6 = addrs.iter().any(|ip| match ip {
            IpAddr::V6(v6) => v6.to_ipv4_mapped().is_none(),
            IpAddr::V4(_) => false,
        });

        if has_ipv6 { "[::]:0" } else { "0.0.0.0:0" }
    }

    // Every resolved address becomes a candidate, in the order given. Dropping one
    // on a guess about which family works would strand a peer whose only address is
    // the dropped one.
    pub fn dial_targets(addrs: &[IpAddr], port: u16, bind: &str) -> Vec<SocketAddr> {
        let v6_socket = bind.starts_with('[');

        addrs
            .iter()
            .map(|ip| match (ip, v6_socket) {
                (IpAddr::V4(v4), true) => {
                    SocketAddr::new(IpAddr::V6(v4.to_ipv6_mapped()), port)
                }
                _ => SocketAddr::new(*ip, port),
            })
            .collect()
    }

    // Builds the s2n-quic client endpoint with the issued peer credential.
    async fn build_client(&self, bind: &str) -> Result<Client, anyhow::Error> {
        let provider = common::rustls::MtlsProvider::new_from_vec(
            self.ca_pem.clone(),
            self.cert_pem.clone(),
            self.key_pem.clone(),
        )
        .await
        .context("build peer mTLS provider")?;

        let dg_endpoint = common::s2n_quic::provider::datagram::default::Endpoint::builder()
            .with_send_capacity(1024)
            .context("send cap")?
            .with_recv_capacity(1024)
            .context("recv cap")?
            .build()
            .map_err(|e| anyhow::anyhow!("build datagram endpoint: {e}"))?;

        let client = Client::builder()
            .with_tls(provider)
            .context("client tls")?
            .with_io(bind)
            .context("client io")?
            .with_datagram(dg_endpoint)
            .context("client datagram")?
            .start()
            .map_err(|e| anyhow::anyhow!("start quic client: {e}"))?;

        Ok(client)
    }

    // Walks the candidates in order and returns the first completed handshake. The
    // winning address is logged: a peer reached over IPv6 is the signal that the
    // dual-stack path is carrying real traffic.
    async fn connect_first_available(
        client: &Client,
        targets: &[SocketAddr],
        server_name: &str,
    ) -> Result<common::s2n_quic::Connection, anyhow::Error> {
        let mut last_error: Option<anyhow::Error> = None;

        for target in targets {
            let connect = Connect::new(*target).with_server_name(server_name.to_string());
            match client.connect(connect).await {
                Ok(connection) => {
                    tracing::debug!("relay peer handshake succeeded on {}", target);
                    return Ok(connection);
                }
                Err(e) => {
                    tracing::debug!("relay peer handshake failed on {}: {}", target, e);
                    last_error = Some(anyhow::Error::new(e));
                }
            }
        }

        Err(last_error
            .unwrap_or_else(|| anyhow::anyhow!("no peer addresses were available"))
            .context("peer connect"))
    }

    // Dials the peer and runs the read/write pump until the connection closes.
    // Inbound datagrams are deserialized and handed to the GATED `ingest`
    // (`PeerManager::ingest`, tagged `FromPeer`): the same presence-proof gate the
    // acceptor path applies — an un-proven peer's AUDIO is dropped fail-closed.
    // Queued outbound `RelayedPacket`s are serialized and sent as datagrams.
    pub async fn run(
        &self,
        addrs: Vec<IpAddr>,
        port: u16,
        server_name: String,
        ingest: Arc<dyn GatedPeerIngest>,
        mut outbound_rx: mpsc::Receiver<RelayedPacket>,
    ) -> Result<(), anyhow::Error> {
        let bind = Self::bind_address(&addrs);
        let targets = Self::dial_targets(&addrs, port, bind);
        if targets.is_empty() {
            return Err(anyhow::anyhow!("peer resolved to no addresses"));
        }

        let client = self.build_client(bind).await?;
        let mut connection = Self::connect_first_available(&client, &targets, &server_name).await?;
        connection.keep_alive(true).ok();
        let connection = Arc::new(connection);

        let recv_conn = connection.clone();
        let ingest_task = ingest.clone();
        let reader = tokio::spawn(async move {
            loop {
                let result = (RecvDatagram { conn: &recv_conn }).await;
                match result {
                    Ok(bytes) => {
                        if let Ok(packet) = QuicNetworkPacket::from_datagram(&bytes) {
                            ingest_task.ingest_from_peer(packet).await;
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        let send_conn = connection.clone();
        while let Some(relayed) = outbound_rx.recv().await {
            if let Ok(bytes) = relayed.packet.to_datagram() {
                let _ = send_conn.datagram_mut(
                    |sender: &mut common::s2n_quic::provider::datagram::default::Sender| {
                        sender.send_datagram(bytes.into())
                    },
                );
            }
        }

        reader.abort();
        Ok(())
    }
}
