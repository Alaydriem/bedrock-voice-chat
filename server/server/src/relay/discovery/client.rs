use std::sync::Arc;

use anyhow::{Context, anyhow};
use common::response::ApiConfigResponse;
use common::structs::relay::{OfferRequest, PeerCertResponse, PeerRedeemRequest, RelayEndpoint};
use reqwest::Client;

// Peer-to-peer relay HTTP client. After discovery moved onto the in-realm
// `!bvca` announce, this no longer talks to any central relay: every call
// targets an explicit peer `host:port` (the peer's advertised HTTPS endpoint).
#[derive(Clone)]
pub struct RelayClient {
    http: Client,
}

impl RelayClient {
    pub fn new() -> Self {
        // Peers present their own (self-signed) certs — each BVC server is its own
        // CA — so cert verification is relaxed; still HTTPS only, never plaintext.
        let http = Client::builder()
            .danger_accept_invalid_certs(true)
            .https_only(true)
            .build()
            .expect("build peer relay reqwest client");
        Self { http }
    }

    pub fn new_shared() -> Arc<Self> {
        Arc::new(Self::new())
    }

    // Divines a peer's QUIC port from its public HTTPS endpoint. Advertised relay
    // endpoints carry the HTTPS port; the QUIC datagram port is read on demand
    // from the unauthenticated `GET /api/config` route, which returns
    // `ApiConfigResponse { quic_port, .. }`.
    //
    // The fetch uses a relaxed transport posture (HTTPS, cert verification
    // bypassed): each BVC server is its own CA, so the caller has no a-priori
    // trust anchor for the peer's HTTPS cert, and `/api/config` carries no secret.
    pub async fn resolve_quic_port(host: &str, http_port: u16) -> Result<u16, anyhow::Error> {
        let bootstrap = Client::builder()
            .danger_accept_invalid_certs(true)
            .https_only(true)
            .build()
            .context("build config bootstrap client")?;
        let url = format!("https://{}:{}/api/config", host, http_port);
        let resp = bootstrap.get(&url).send().await.context("config request")?;
        if !resp.status().is_success() {
            return Err(anyhow!("config returned {}", resp.status()));
        }
        let config: ApiConfigResponse = resp.json().await.context("decode config response")?;
        u16::try_from(config.quic_port)
            .map_err(|_| anyhow!("config quic_port {} out of range", config.quic_port))
    }

    // HTTP-pulls a discovered `.opus` from a responding peer's public audio
    // endpoint with a single-use stream token. `host`/`http_port` are the
    // responder's advertised HTTPS endpoint directly (the relay endpoint IS the
    // HTTP endpoint), so no port override is needed. The body is read chunk by
    // chunk via `Response::chunk()` and accumulated, so the caller can race the
    // read against a cancellation token and drop it promptly mid-transfer.
    //
    // Relaxed transport posture (HTTPS, cert verification bypassed): each BVC
    // server is its own CA, so the caller has no a-priori trust anchor for the
    // responder's HTTPS cert. The security root is the authorized peer link that
    // carried the token, not this read; the token is short-lived and single-use,
    // so it is NOT retried here — a retry must re-issue discovery for a fresh token.
    pub async fn pull_audio(
        host: &str,
        http_port: u16,
        token: &str,
    ) -> Result<Vec<u8>, anyhow::Error> {
        let client = Client::builder()
            .danger_accept_invalid_certs(true)
            .https_only(true)
            .build()
            .context("build audio pull client")?;
        let url = format!(
            "https://{}:{}/api/audio/stream?token={}",
            host, http_port, token
        );
        let mut resp = client
            .get(&url)
            .send()
            .await
            .context("audio pull request")?;
        if !resp.status().is_success() {
            return Err(anyhow!("audio pull returned {}", resp.status()));
        }
        let mut buf = Vec::new();
        while let Some(chunk) = resp.chunk().await.context("audio pull body chunk")? {
            buf.extend_from_slice(&chunk);
        }
        Ok(buf)
    }

    // Asks the minter to mint a peer code for `hashed_world` bound to OUR
    // endpoint and inject it into the realm. The code travels back through the
    // realm to our client — never this HTTP response — so a 2xx only confirms the
    // offer was accepted. Targets the minter's HTTPS endpoint directly.
    pub async fn offer(
        &self,
        minter_host: &str,
        minter_http_port: u16,
        hashed_world: &str,
        asker: &RelayEndpoint,
        asker_public_key: Vec<u8>,
    ) -> Result<(), anyhow::Error> {
        let body = OfferRequest {
            hashed_world: hashed_world.to_string(),
            asker_host: asker.host.clone(),
            asker_port: asker.port,
            asker_public_key,
        };
        let url = format!(
            "https://{}:{}/api/relay/offer",
            minter_host, minter_http_port
        );
        let resp = self
            .http
            .post(&url)
            .json(&body)
            .send()
            .await
            .context("offer request")?;
        if !resp.status().is_success() {
            return Err(anyhow!("offer returned {}", resp.status()));
        }
        Ok(())
    }

    // Redeems a code (observed through the realm) at the minter for the in-memory
    // peer cert. `presenter` must match the code's bound recipient (our endpoint).
    pub async fn peer_redeem(
        &self,
        minter_host: &str,
        minter_http_port: u16,
        code: &str,
        presenter: &RelayEndpoint,
    ) -> Result<PeerCertResponse, anyhow::Error> {
        let body = PeerRedeemRequest {
            code: code.to_string(),
            presenter_host: presenter.host.clone(),
            presenter_port: presenter.port,
        };
        let url = format!(
            "https://{}:{}/api/relay/peer-redeem",
            minter_host, minter_http_port
        );
        let resp = self
            .http
            .post(&url)
            .json(&body)
            .send()
            .await
            .context("peer-redeem request")?;
        if !resp.status().is_success() {
            return Err(anyhow!("peer-redeem returned {}", resp.status()));
        }
        resp.json::<PeerCertResponse>()
            .await
            .context("decode peer-redeem response")
    }
}

impl Default for RelayClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Serves a single HTTPS request with a self-signed cert and replies with the
    // given JSON body on `GET /api/config`. Returns the bound port. This mirrors
    // a relaxed TLS posture: the served cert is self-signed
    // and the client accepts it without a trust anchor.
    struct OneShotConfigServer;

    impl OneShotConfigServer {
        fn spawn(body: String) -> u16 {
            use std::io::{Read, Write};
            use std::net::TcpListener;
            use std::sync::Arc;

            let cert = rcgen::generate_simple_self_signed(vec!["127.0.0.1".to_string()])
                .expect("generate self-signed cert");
            let cert_der = rustls::pki_types::CertificateDer::from(cert.cert.der().to_vec());
            let key_der =
                rustls::pki_types::PrivateKeyDer::try_from(cert.signing_key.serialize_der())
                    .expect("private key der");

            let tls_config = rustls::ServerConfig::builder()
                .with_no_client_auth()
                .with_single_cert(vec![cert_der], key_der)
                .expect("server tls config");
            let tls_config = Arc::new(tls_config);

            let listener = TcpListener::bind("127.0.0.1:0").expect("bind listener");
            let port = listener.local_addr().expect("local addr").port();

            std::thread::spawn(move || {
                let (mut stream, _) = listener.accept().expect("accept");
                let mut conn =
                    rustls::ServerConnection::new(tls_config).expect("server connection");

                let mut request = Vec::new();
                loop {
                    if conn.wants_read() {
                        if conn.read_tls(&mut stream).unwrap_or(0) == 0 {
                            break;
                        }
                        conn.process_new_packets().expect("process packets");
                        let mut buf = [0u8; 4096];
                        if let Ok(n) = conn.reader().read(&mut buf) {
                            request.extend_from_slice(&buf[..n]);
                        }
                    }
                    if conn.wants_write() {
                        conn.write_tls(&mut stream).expect("write tls handshake");
                    }
                    if !conn.is_handshaking() && request.windows(4).any(|w| w == b"\r\n\r\n") {
                        break;
                    }
                }

                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                conn.writer()
                    .write_all(response.as_bytes())
                    .expect("queue response");
                while conn.wants_write() {
                    conn.write_tls(&mut stream).expect("write tls response");
                }
                conn.send_close_notify();
                let _ = conn.write_tls(&mut stream);
            });

            port
        }
    }

    // Serves a single HTTPS request with a self-signed cert and replies with the
    // given binary body on any `GET`. Returns the bound port. Used to exercise
    // `pull_audio` against a real TLS endpoint without a full Rocket server.
    struct OneShotBytesServer;

    impl OneShotBytesServer {
        fn spawn(body: Vec<u8>) -> u16 {
            use std::io::{Read, Write};
            use std::net::TcpListener;
            use std::sync::Arc;

            let cert = rcgen::generate_simple_self_signed(vec!["127.0.0.1".to_string()])
                .expect("generate self-signed cert");
            let cert_der = rustls::pki_types::CertificateDer::from(cert.cert.der().to_vec());
            let key_der =
                rustls::pki_types::PrivateKeyDer::try_from(cert.signing_key.serialize_der())
                    .expect("private key der");

            let tls_config = rustls::ServerConfig::builder()
                .with_no_client_auth()
                .with_single_cert(vec![cert_der], key_der)
                .expect("server tls config");
            let tls_config = Arc::new(tls_config);

            let listener = TcpListener::bind("127.0.0.1:0").expect("bind listener");
            let port = listener.local_addr().expect("local addr").port();

            std::thread::spawn(move || {
                let (mut stream, _) = listener.accept().expect("accept");
                let mut conn =
                    rustls::ServerConnection::new(tls_config).expect("server connection");

                let mut request = Vec::new();
                loop {
                    if conn.wants_read() {
                        if conn.read_tls(&mut stream).unwrap_or(0) == 0 {
                            break;
                        }
                        conn.process_new_packets().expect("process packets");
                        let mut buf = [0u8; 4096];
                        if let Ok(n) = conn.reader().read(&mut buf) {
                            request.extend_from_slice(&buf[..n]);
                        }
                    }
                    if conn.wants_write() {
                        conn.write_tls(&mut stream).expect("write tls handshake");
                    }
                    if !conn.is_handshaking() && request.windows(4).any(|w| w == b"\r\n\r\n") {
                        break;
                    }
                }

                let header = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                let mut response = header.into_bytes();
                response.extend_from_slice(&body);
                conn.writer().write_all(&response).expect("queue response");
                while conn.wants_write() {
                    conn.write_tls(&mut stream).expect("write tls response");
                }
                conn.send_close_notify();
                let _ = conn.write_tls(&mut stream);
            });

            port
        }
    }

    #[tokio::test]
    async fn pull_audio_reads_full_body_from_served_endpoint() {
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

        let body: Vec<u8> = (0..4096u32).map(|i| (i % 251) as u8).collect();
        let port = OneShotBytesServer::spawn(body.clone());

        let pulled = RelayClient::pull_audio("127.0.0.1", port, "any-token")
            .await
            .expect("pull_audio should succeed");

        assert_eq!(pulled, body);
    }

    #[tokio::test]
    async fn resolve_quic_port_reads_quic_port_from_served_config() {
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

        let body = serde_json::json!({
            "status": "Ok",
            "client_id": "",
            "protocol_version": "1.3.0",
            "quic_port": 5443u32,
            "spatial_audio": {}
        })
        .to_string();

        let port = OneShotConfigServer::spawn(body);

        let resolved = RelayClient::resolve_quic_port("127.0.0.1", port)
            .await
            .expect("resolve_quic_port should succeed");

        assert_eq!(resolved, 5443);
    }

    #[tokio::test]
    async fn offer_posts_to_minter_and_accepts_success() {
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
        // The minter replies 2xx; the code itself rides the realm, not the response.
        let port = OneShotConfigServer::spawn(String::new());
        let client = RelayClient::new();
        let asker = RelayEndpoint {
            host: "asker.host".into(),
            port: 6000,
            primary: false,
        };
        client
            .offer("127.0.0.1", port, "W", &asker, vec![0u8; 32])
            .await
            .expect("offer should be accepted");
    }

    #[tokio::test]
    async fn peer_redeem_parses_cert_response() {
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
        let body = serde_json::json!({
            "ca_pem": "CA",
            "cert_pem": "CERT",
            "key_pem": "KEY"
        })
        .to_string();
        let port = OneShotConfigServer::spawn(body);
        let client = RelayClient::new();
        let presenter = RelayEndpoint {
            host: "asker.host".into(),
            port: 6000,
            primary: false,
        };
        let resp = client
            .peer_redeem("127.0.0.1", port, "the-code", &presenter)
            .await
            .expect("peer-redeem should parse the cert response");
        assert_eq!(resp.ca_pem, "CA");
        assert_eq!(resp.cert_pem, "CERT");
        assert_eq!(resp.key_pem, "KEY");
    }
}
