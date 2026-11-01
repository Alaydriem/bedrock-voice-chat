use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use common::curia;
use hickory_server::Server;
use tokio::net::{TcpListener, UdpSocket};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use super::dns_handler::PlayDnsHandler;
use super::error::PlayError;
use super::zone::PlayZone;

// The play zone's DNS listeners, UDP and TCP on every configured address.
pub struct PlayDnsServer {
    server: Server<PlayDnsHandler>,
    addrs: Vec<SocketAddr>,
}

impl PlayDnsServer {
    // A TCP connection that sends nothing for this long is closed.
    const TCP_TIMEOUT: Duration = Duration::from_secs(5);
    // Bytes of outgoing responses buffered per TCP connection.
    const TCP_BUFFER: usize = 1 << 16;
    // Draws of an ephemeral port before giving up on finding one free for both protocols.
    const EPHEMERAL_ATTEMPTS: usize = 16;

    // Fails on the first address that cannot be bound, naming it. The registry does not
    // run without DNS: a stub resolver commonly holds 127.0.0.53:53, and carrying on
    // would leave every play name unresolvable behind a healthy-looking process.
    pub async fn bind(zone: Arc<PlayZone>, binds: &[SocketAddr]) -> Result<Self, PlayError> {
        let handler = PlayDnsHandler::new(zone.clone(), zone.dns().clone());
        let mut server = Server::new(handler);
        let mut addrs = Vec::with_capacity(binds.len());

        for address in binds {
            let (udp, tcp, bound) = Self::bind_pair(*address).await?;

            server.register_socket(udp);
            server.register_listener(tcp, Self::TCP_TIMEOUT, Self::TCP_BUFFER);
            addrs.push(bound);
        }

        Ok(Self { server, addrs })
    }

    // UDP and TCP on one address. With port 0, TCP takes the port UDP got, so the pair is
    // one address; a port UDP got but TCP cannot have is drawn again.
    async fn bind_pair(
        address: SocketAddr,
    ) -> Result<(UdpSocket, TcpListener, SocketAddr), PlayError> {
        let attempts = if address.port() == 0 { Self::EPHEMERAL_ATTEMPTS } else { 1 };
        let mut last = None;

        for _ in 0..attempts {
            let udp = UdpSocket::bind(address).await.map_err(|source| PlayError::Bind {
                address,
                source,
            })?;
            let bound = udp.local_addr().map_err(|source| PlayError::Bind {
                address,
                source,
            })?;

            match TcpListener::bind(bound).await {
                Ok(tcp) => return Ok((udp, tcp, bound)),
                Err(source) => last = Some(PlayError::Bind { address: bound, source }),
            }
        }

        Err(last.expect("at least one attempt was made"))
    }

    // The addresses actually bound, for tests that ask for port 0.
    pub fn local_addrs(&self) -> Vec<SocketAddr> {
        self.addrs.clone()
    }

    pub fn spawn(mut self, cancel: CancellationToken) -> JoinHandle<()> {
        let shutdown = self.server.shutdown_token().clone();

        tokio::spawn(async move {
            tokio::spawn(async move {
                cancel.cancelled().await;
                shutdown.cancel();
            });

            if let Err(e) = self.server.block_until_done().await {
                curia::error!("the play DNS server stopped", { "error": e.to_string() });
            }
        })
    }
}
