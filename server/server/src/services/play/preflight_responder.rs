use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use common::curia;
use rcgen::{CertificateParams, KeyPair};
use rustls::ServerConfig;
use rustls::pki_types::{PrivateKeyDer, PrivatePkcs8KeyDer};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::oneshot;
use tokio::task::{JoinHandle, JoinSet};
use tokio_rustls::TlsAcceptor;

use crate::demux::PublicListener;
use crate::services::CurrentNonce;

/// Answers the registry's reachability probe before the server's own listeners exist.
///
/// It binds the public port, serves the nonce the registry minted and nothing else, and
/// gives the port back on `stop` so the listeners can bind it. The certificate is
/// self-signed and fresh: the probe checks the nonce, not the certificate.
pub struct PlayPreflightResponder {
    local_addr: SocketAddr,
    shutdown: oneshot::Sender<()>,
    task: JoinHandle<()>,
}

impl PlayPreflightResponder {
    pub const PATH: &'static str = "/health/enrollment-nonce";

    const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);
    const MAX_REQUEST: usize = 4096;
    const ACCEPT_RETRY: Duration = Duration::from_millis(50);
    const MAX_CONNECTIONS: usize = 32;

    pub async fn start(listen: SocketAddr, nonce: Arc<CurrentNonce>) -> Result<Self> {
        let acceptor = Self::acceptor()?;
        let listener = PublicListener::bind(listen)
            .with_context(|| format!("binding {listen} to answer the registry's preflight"))?;
        let local_addr = listener.local_addr()?;
        let (shutdown, stop) = oneshot::channel();
        let task = tokio::spawn(Self::serve(listener, acceptor, nonce, stop));

        Ok(Self {
            local_addr,
            shutdown,
            task,
        })
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// Stops accepting, closes every open connection, and returns once the port is free.
    pub async fn stop(self) {
        let _ = self.shutdown.send(());
        let _ = self.task.await;
    }

    fn acceptor() -> Result<TlsAcceptor> {
        let key = KeyPair::generate().context("generating the responder key")?;
        let certificate = CertificateParams::new(vec!["play-preflight.invalid".to_string()])
            .context("building the responder certificate")?
            .self_signed(&key)
            .context("signing the responder certificate")?;

        let mut config = ServerConfig::builder_with_provider(Arc::new(
            rustls::crypto::aws_lc_rs::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .map_err(|e| anyhow!("selecting TLS versions: {e}"))?
        .with_no_client_auth()
        .with_single_cert(
            vec![certificate.der().clone()],
            PrivateKeyDer::from(PrivatePkcs8KeyDer::from(key.serialize_der())),
        )
        .map_err(|e| anyhow!("installing the responder certificate: {e}"))?;
        config.alpn_protocols = vec![b"http/1.1".to_vec()];

        Ok(TlsAcceptor::from(Arc::new(config)))
    }

    async fn serve(
        listener: TcpListener,
        acceptor: TlsAcceptor,
        nonce: Arc<CurrentNonce>,
        mut stop: oneshot::Receiver<()>,
    ) {
        let mut connections = JoinSet::new();
        let mut last_error: Option<String> = None;
        loop {
            tokio::select! {
                _ = &mut stop => break,
                accepted = listener.accept() => match accepted {
                    Ok((stream, _)) => {
                        last_error = None;
                        // A flood past the cap is dropped, so it cannot use up the
                        // descriptors the listeners need once the probe is done.
                        if connections.len() < Self::MAX_CONNECTIONS {
                            connections.spawn(Self::handle(stream, acceptor.clone(), nonce.clone()));
                        }
                    }
                    Err(e) => {
                        let error = e.to_string();
                        if last_error.as_deref() != Some(error.as_str()) {
                            curia::warn!("Play preflight responder could not accept a connection.", { "error": error.clone() });
                            last_error = Some(error);
                        }
                        tokio::time::sleep(Self::ACCEPT_RETRY).await;
                    }
                },
                Some(_) = connections.join_next(), if !connections.is_empty() => {}
            }
        }

        // Open connections hold the port on some platforms, so they end before this returns.
        connections.abort_all();
        while connections.join_next().await.is_some() {}
    }

    async fn handle(stream: TcpStream, acceptor: TlsAcceptor, nonce: Arc<CurrentNonce>) {
        let _ = tokio::time::timeout(Self::REQUEST_TIMEOUT, Self::respond(stream, acceptor, nonce))
            .await;
    }

    async fn respond(
        stream: TcpStream,
        acceptor: TlsAcceptor,
        nonce: Arc<CurrentNonce>,
    ) -> io::Result<()> {
        let mut tls = acceptor.accept(stream).await?;

        let mut request = Vec::new();
        let mut chunk = [0u8; 1024];
        while !request.windows(4).any(|window| window == b"\r\n\r\n")
            && request.len() < Self::MAX_REQUEST
        {
            let read = tls.read(&mut chunk).await?;
            if read == 0 {
                break;
            }
            request.extend_from_slice(&chunk[..read]);
        }

        tls.write_all(&Self::response(&request, &nonce)).await?;
        tls.shutdown().await
    }

    fn response(request: &[u8], nonce: &CurrentNonce) -> Vec<u8> {
        let line = request.split(|byte| *byte == b'\n').next().unwrap_or_default();
        let line = String::from_utf8_lossy(line);
        let mut parts = line.split_whitespace();
        let asked = parts.next() == Some("GET") && parts.next() == Some(Self::PATH);

        match nonce.get().filter(|_| asked) {
            Some(value) => format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{value}",
                value.len()
            )
            .into_bytes(),
            None => b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                .to_vec(),
        }
    }
}
