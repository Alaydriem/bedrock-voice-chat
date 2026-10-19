//! A WebSocket client that presents a player certificate, as the app's Rust side does.

use std::sync::Arc;

use anyhow::{Result, anyhow};
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, ServerName};
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;
use tokio_rustls::client::TlsStream;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::Error as WsError;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;

pub type MtlsSocket = WebSocketStream<TlsStream<TcpStream>>;

pub struct MtlsWebSocket;

impl MtlsWebSocket {
    /// Opens `wss://<base>/<path>`. `Err(status)` is the HTTP status that refused the upgrade.
    pub async fn connect(
        base_url: &str,
        ca_pem: &str,
        cert_pem: &str,
        key_pem: &str,
        path: &str,
    ) -> std::result::Result<MtlsSocket, u16> {
        let authority = base_url.trim_start_matches("https://").to_string();
        let tls = Self::tls(&authority, ca_pem, cert_pem, key_pem)
            .await
            .expect("TLS to the test server");
        let request = format!("wss://{authority}{path}")
            .into_client_request()
            .expect("websocket request");
        match tokio_tungstenite::client_async(request, tls).await {
            Ok((socket, _)) => Ok(socket),
            Err(WsError::Http(response)) => Err(response.status().as_u16()),
            Err(e) => panic!("websocket upgrade failed: {e}"),
        }
    }

    async fn tls(
        authority: &str,
        ca_pem: &str,
        cert_pem: &str,
        key_pem: &str,
    ) -> Result<TlsStream<TcpStream>> {
        let mut roots = rustls::RootCertStore::empty();
        for cert in CertificateDer::pem_slice_iter(ca_pem.as_bytes()) {
            roots.add(cert?)?;
        }
        let chain: Vec<CertificateDer<'static>> =
            CertificateDer::pem_slice_iter(cert_pem.as_bytes()).collect::<Result<_, _>>()?;
        let key = PrivateKeyDer::from_pem_slice(key_pem.as_bytes())?;

        let mut config = rustls::ClientConfig::builder_with_provider(Arc::new(
            rustls::crypto::aws_lc_rs::default_provider(),
        ))
        .with_safe_default_protocol_versions()?
        .with_root_certificates(roots)
        .with_client_auth_cert(chain, key)?;
        config.alpn_protocols = vec![b"http/1.1".to_vec()];

        let host = authority
            .split(':')
            .next()
            .ok_or_else(|| anyhow!("no host in {authority}"))?
            .to_string();
        let stream = TcpStream::connect(authority).await?;
        let name = ServerName::try_from(host)?;
        Ok(TlsConnector::from(Arc::new(config)).connect(name, stream).await?)
    }
}
