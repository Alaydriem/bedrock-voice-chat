use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{anyhow, Context};
use common::response::ApiConfigResponse;
use common::structs::relay::{
    LookupRequest, LookupResponse, PeerCertRequest, PeerCertResponse, RegisterChallengeRequest,
    RegisterChallengeResponse, RegisterRequest, RelayEndpoint,
};
use reqwest::Client;

use common::tls::SpkiPinningVerifier;

#[derive(Clone)]
pub struct RelayClient {
    base_url: String,
    http: Client,
}

impl RelayClient {
    // Default production discovery relay. Used when the operator sets no
    // `features.relay.client_url` override.
    pub const DEFAULT_RELAY_URL: &str = "https://relay.bedrockvoicechat.com";

    // Baked-in SPKI pin (base64 SHA-256 of the SubjectPublicKeyInfo) for the
    // default relay above. When empty, connecting to the default relay errors;
    // connect to a custom `client_url` instead.
    pub const DEFAULT_RELAY_SPKI_PIN: &str = "Ae0qBh6ONPl2sGUMgJiRJE9mrho9ehVfPBPv3kI5eEo=";

    pub fn new(base_url: &str) -> Result<Self, anyhow::Error> {
        let trimmed = base_url.trim_end_matches('/');
        let is_default = trimmed == Self::DEFAULT_RELAY_URL;

        let http = if is_default {
            // Default production relay: pin its certificate.
            if Self::DEFAULT_RELAY_SPKI_PIN.is_empty() {
                return Err(anyhow!(
                    "default relay {} has no provisioned SPKI pin yet; set features.relay.client_url to a custom relay to test, or configure DEFAULT_RELAY_SPKI_PIN",
                    Self::DEFAULT_RELAY_URL
                ));
            }
            let verifier =
                Arc::new(SpkiPinningVerifier::new(&[Self::DEFAULT_RELAY_SPKI_PIN.to_string()]));
            let provider = verifier.provider();
            let config = rustls::ClientConfig::builder_with_provider(provider)
                .with_safe_default_protocol_versions()
                .context("rustls default protocol versions")?
                .dangerous()
                .with_custom_certificate_verifier(verifier)
                .with_no_client_auth();
            Client::builder()
                .use_preconfigured_tls(config)
                .https_only(true)
                .build()
                .context("build pinned relay reqwest client")?
        } else {
            // Operator-configured non-default relay: we don't bake their cert, so
            // SPKI pinning is removed. Still HTTPS only (never plaintext); cert
            // verification is relaxed because a self-hosted relay typically uses
            // its own/self-signed cert and the operator opted into this endpoint.
            tracing::warn!(
                base_url = %trimmed,
                "RelayClient: non-default relay — SPKI pinning disabled (HTTPS, cert verification relaxed)"
            );
            Client::builder()
                .danger_accept_invalid_certs(true)
                .https_only(true)
                .build()
                .context("build non-default relay reqwest client")?
        };

        Ok(Self {
            base_url: trimmed.to_string(),
            http,
        })
    }

    pub fn new_shared(base_url: &str) -> Result<Arc<Self>, anyhow::Error> {
        Ok(Arc::new(Self::new(base_url)?))
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    pub async fn register(
        &self,
        hashed_world: &str,
        endpoint: RelayEndpoint,
        ttl_secs: u32,
        token: &str,
    ) -> Result<(), anyhow::Error> {
        let body = RegisterRequest {
            hashed_world: hashed_world.to_string(),
            endpoint,
            ttl_secs,
            token: token.to_string(),
        };
        let resp = self
            .http
            .post(self.url("/relay/register"))
            .json(&body)
            .send()
            .await
            .context("relay register request")?;
        if !resp.status().is_success() {
            return Err(anyhow!("relay register returned {}", resp.status()));
        }
        Ok(())
    }

    // First leg of endpoint-control-proven registration. Asks the relay for a
    // challenge bound to `endpoint`; returns the `(token, nonce)`.
    // The caller must serve `nonce` at its own `/relay/proof/<nonce>` route so
    // the relay's reachability callback can confirm control, then present `token`
    // to `register`.
    pub async fn request_challenge(
        &self,
        endpoint: RelayEndpoint,
    ) -> Result<RegisterChallengeResponse, anyhow::Error> {
        let body = RegisterChallengeRequest { endpoint };
        let resp = self
            .http
            .post(self.url("/relay/challenge"))
            .json(&body)
            .send()
            .await
            .context("relay challenge request")?;
        if !resp.status().is_success() {
            return Err(anyhow!("relay challenge returned {}", resp.status()));
        }
        resp.json::<RegisterChallengeResponse>()
            .await
            .context("decode challenge response")
    }

    // Fetches an in-memory peer client cert from the ACCEPTOR. Targets the remote
    // peer server's HTTPS peer-cert route (NOT the discovery relay): the acceptor
    // signs and returns a cert for the initiator's `host:{https_port}` identity
    // ONLY when the initiator is mutually presence-proven for `hashed_world`.
    //
    // Each BVC server is its own CA, so the initiator has no a-priori trust anchor
    // for the acceptor's HTTPS cert — the acceptor's CA arrives IN this response,
    // and the subsequent QUIC dial validates mTLS against it. Issuance is gated on
    // the mutual presence proof, so this bootstrap fetch is made with transport
    // verification relaxed.
    pub async fn fetch_peer_cert(
        &self,
        initiator: &RelayEndpoint,
        acceptor_host: &str,
        acceptor_http_port: u16,
        hashed_world: &str,
    ) -> Result<PeerCertResponse, anyhow::Error> {
        let body = PeerCertRequest {
            host: initiator.host.clone(),
            port: initiator.port,
            hashed_world: hashed_world.to_string(),
        };
        let bootstrap = Client::builder()
            .danger_accept_invalid_certs(true)
            .build()
            .context("build peer-cert bootstrap client")?;
        let url = format!(
            "https://{}:{}/relay/peer-cert",
            acceptor_host, acceptor_http_port
        );
        let resp = bootstrap
            .post(&url)
            .json(&body)
            .send()
            .await
            .context("peer-cert request")?;
        if !resp.status().is_success() {
            return Err(anyhow!("peer-cert returned {}", resp.status()));
        }
        resp.json::<PeerCertResponse>()
            .await
            .context("decode peer-cert response")
    }

    // Divines a peer's QUIC port from its public HTTPS endpoint. Advertised relay
    // endpoints carry the HTTPS port; the QUIC datagram port is read on demand
    // from the unauthenticated `GET /api/config` route, which returns
    // `ApiConfigResponse { quic_port, .. }`.
    //
    // The fetch uses the same relaxed transport posture as `fetch_peer_cert`:
    // each BVC server is its own CA, so the caller has no a-priori trust anchor
    // for the peer's HTTPS cert, and `/api/config` carries no secret. The security
    // root for cross-server voice remains the mutual presence proof gating
    // peer-cert issuance, not this read.
    pub async fn resolve_quic_port(host: &str, http_port: u16) -> Result<u16, anyhow::Error> {
        let bootstrap = Client::builder()
            .danger_accept_invalid_certs(true)
            .https_only(true)
            .build()
            .context("build config bootstrap client")?;
        let url = format!("https://{}:{}/api/config", host, http_port);
        let resp = bootstrap
            .get(&url)
            .send()
            .await
            .context("config request")?;
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
    // Same relaxed transport posture as `fetch_peer_cert`: each BVC server is its
    // own CA, so the caller has no a-priori trust anchor for the responder's HTTPS
    // cert. The security root is the presence-proven peer link that carried the
    // token, not this read; the token is short-lived and single-use, so it is NOT
    // retried here — a retry must re-issue discovery for a fresh token.
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
        let mut resp = client.get(&url).send().await.context("audio pull request")?;
        if !resp.status().is_success() {
            return Err(anyhow!("audio pull returned {}", resp.status()));
        }
        let mut buf = Vec::new();
        while let Some(chunk) = resp.chunk().await.context("audio pull body chunk")? {
            buf.extend_from_slice(&chunk);
        }
        Ok(buf)
    }

    // Looks up peers for `hashed_worlds`, scoped to the caller. `token` is the
    // endpoint-control-proven token for `caller`: the relay rejects the lookup
    // unless the caller controls the endpoint it claims.
    pub async fn lookup(
        &self,
        caller: RelayEndpoint,
        hashed_worlds: &[String],
        token: &str,
    ) -> Result<HashMap<String, Vec<RelayEndpoint>>, anyhow::Error> {
        let body = LookupRequest {
            caller,
            hashed_worlds: hashed_worlds.to_vec(),
            token: token.to_string(),
        };
        let resp = self
            .http
            .post(self.url("/relay/lookup"))
            .json(&body)
            .send()
            .await
            .context("relay lookup request")?;
        if !resp.status().is_success() {
            return Err(anyhow!("relay lookup returned {}", resp.status()));
        }
        let parsed: LookupResponse = resp.json().await.context("decode lookup response")?;
        Ok(parsed.worlds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custom_relay_builds_unpinned_https() {
        // A non-default (operator-configured) relay builds with SPKI pinning
        // removed but still HTTPS — the local/dev + bring-your-own-relay path.
        let client = RelayClient::new("https://relay.example.com");
        assert!(client.is_ok());
    }

    #[test]
    fn default_relay_requires_provisioned_pin() {
        // The default production relay must be pinned; while DEFAULT_RELAY_SPKI_PIN
        // is unprovisioned (empty) we refuse to connect unpinned.
        let client = RelayClient::new(RelayClient::DEFAULT_RELAY_URL);
        assert_eq!(client.is_err(), RelayClient::DEFAULT_RELAY_SPKI_PIN.is_empty());
    }

    #[test]
    fn trims_trailing_slash_from_base_url() {
        let client = RelayClient::new("https://relay.example.com/").unwrap();
        assert_eq!(client.url("/relay/register"), "https://relay.example.com/relay/register");
    }

    // Serves a single HTTPS request with a self-signed cert and replies with the
    // given JSON body on `GET /api/config`. Returns the bound port. This mirrors
    // the relaxed-TLS posture of `fetch_peer_cert`: the served cert is self-signed
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
            let key_der = rustls::pki_types::PrivateKeyDer::try_from(
                cert.signing_key.serialize_der(),
            )
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
                    if !conn.is_handshaking()
                        && request.windows(4).any(|w| w == b"\r\n\r\n")
                    {
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
            let key_der = rustls::pki_types::PrivateKeyDer::try_from(
                cert.signing_key.serialize_der(),
            )
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
                    if !conn.is_handshaking()
                        && request.windows(4).any(|w| w == b"\r\n\r\n")
                    {
                        break;
                    }
                }

                let header = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                let mut response = header.into_bytes();
                response.extend_from_slice(&body);
                conn.writer()
                    .write_all(&response)
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
}
