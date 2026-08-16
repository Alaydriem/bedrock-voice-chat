mod verifier;

pub use verifier::VoiceProbeVerifier;

use std::io::Cursor;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;

use rustls::pki_types::ServerName;
use rustls::{ClientConfig, ClientConnection};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use super::RouteProbe;
use crate::net::NetTimeouts;
use crate::structs::network::VoiceProtocol;
use crate::structs::reachability::{AnsweredVia, ReachabilityOutcome};

/// Whether the WebSocket voice transport is reachable on a server's public TLS port.
///
/// The question the other probes cannot answer. All of them measure UDP or the HTTP API, and
/// a client whose UDP is blocked reaches voice over the same TCP port the API uses — so
/// "HTTPS answered" says the port is open without saying the voice listener is behind it.
///
/// Answered by the ALPN and nothing else. `AlpnDemux` routes a hello offering the voice
/// protocol to the WebSocket listener and refuses it with `no_application_protocol` when
/// there is none, so the negotiated protocol separates a server that carries voice this way
/// from one that only serves the API on that port.
///
/// Credential-free, and no session is created: the socket is dropped as soon as the ALPN is
/// known, which is before the listener would ask for a client certificate.
pub struct WsVoiceProbe;

impl WsVoiceProbe {
    const READ_CHUNK: usize = 4096;

    pub async fn probe(dest: SocketAddr, server_name: &str) -> ReachabilityOutcome {
        if !RouteProbe::is_routable(dest) {
            return ReachabilityOutcome::NoRoute;
        }

        let started = Instant::now();
        match tokio::time::timeout(
            NetTimeouts::VOICE_WEBSOCKET,
            Self::negotiate(dest, server_name),
        )
        .await
        {
            Ok(true) => ReachabilityOutcome::Answered {
                via: AnsweredVia::VoiceWebSocket,
                rtt_micros: started.elapsed().as_micros().min(u32::MAX as u128) as u32,
            },
            Ok(false) | Err(_) => ReachabilityOutcome::Silent,
        }
    }

    /// Drives the handshake only as far as the server naming the voice protocol.
    ///
    /// Every failure is the same answer — this port does not carry voice — so none of them
    /// is distinguished. A refusal, a timeout and a server that speaks plain HTTPS all leave
    /// a client with no fallback path, and the screen showing it has one thing to say.
    async fn negotiate(dest: SocketAddr, server_name: &str) -> bool {
        let Ok(config) = Self::client_config() else {
            return false;
        };
        let Ok(name) = Self::server_name(server_name) else {
            return false;
        };
        let Ok(mut connection) = ClientConnection::new(Arc::new(config), name) else {
            return false;
        };
        let Ok(mut stream) = TcpStream::connect(dest).await else {
            return false;
        };

        let mut chunk = [0u8; Self::READ_CHUNK];

        loop {
            let mut pending = Vec::new();
            while connection.wants_write() {
                if connection.write_tls(&mut pending).is_err() {
                    return false;
                }
            }
            if !pending.is_empty() && stream.write_all(&pending).await.is_err() {
                return false;
            }

            if Self::negotiated_voice(&connection) {
                return true;
            }

            // A completed handshake that never named the protocol is a server serving
            // something else on this port. Nothing further would change that.
            if !connection.is_handshaking() {
                return false;
            }

            let read = match stream.read(&mut chunk).await {
                Ok(0) | Err(_) => return false,
                Ok(read) => read,
            };

            // rustls consumes one record per call, so the chunk is drained into it rather
            // than handed over once.
            let mut cursor = Cursor::new(&chunk[..read]);
            while (cursor.position() as usize) < read {
                match connection.read_tls(&mut cursor) {
                    Ok(0) => break,
                    Ok(_) => {}
                    Err(_) => return false,
                }

                // The protocol is recorded as the flight is processed, so a rejection later
                // in that same flight still leaves the answer behind. Reading it here is
                // what makes a server whose certificate this probe cannot judge report the
                // path it does have.
                if connection.process_new_packets().is_err() {
                    return Self::negotiated_voice(&connection);
                }
            }
        }
    }

    fn negotiated_voice(connection: &ClientConnection) -> bool {
        connection.alpn_protocol() == Some(VoiceProtocol::ALPN)
    }

    /// Offers the voice protocol and nothing else, so the server either names it or refuses.
    ///
    /// Offering a second protocol would let a server that serves only the API negotiate that
    /// one instead, and the refusal this probe reads as its answer would never arrive.
    fn client_config() -> Result<ClientConfig, rustls::Error> {
        let verifier = VoiceProbeVerifier::new_shared();
        let provider = verifier.crypto_provider();

        let mut config = ClientConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()?
            .dangerous()
            .with_custom_certificate_verifier(verifier)
            .with_no_client_auth();

        config.alpn_protocols = vec![VoiceProtocol::ALPN.to_vec()];
        // SNI is the only thing that places this probe on the right backend behind a
        // name-routing proxy, which is how a hosted instance is reached.
        config.enable_sni = true;

        Ok(config)
    }

    // The host as the reachability request carries it, which brackets an IPv6 literal.
    // rustls parses the address itself and rejects the bracketed form.
    fn server_name(host: &str) -> Result<ServerName<'static>, rustls::pki_types::InvalidDnsNameError>
    {
        let unbracketed = host.trim_start_matches('[').trim_end_matches(']');
        ServerName::try_from(unbracketed.to_string())
    }
}
