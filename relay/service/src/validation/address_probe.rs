use std::error::Error;
use std::io::ErrorKind;
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

use common::curia;
use reqwest::Certificate;

use super::probe_outcome::ProbeOutcome;

pub struct LiveAddressProbe {
    // When set, only these roots are trusted and the platform's are not.
    roots: Option<Vec<Certificate>>,
    // The certificate is not checked. A probe that only asks whether something answers.
    accept_any_certificate: bool,
    connect_timeout: Option<Duration>,
    request_timeout: Duration,
}

impl LiveAddressProbe {
    pub const DEFAULT_PORT: u16 = 443;
    const TIMEOUT: Duration = Duration::from_secs(10);
    const PATH: &'static str = "/health/enrollment-nonce";
    // The nonce is 43 characters. A larger body is not a nonce.
    const MAX_BODY: usize = 256;

    pub fn new() -> Self {
        Self {
            roots: None,
            accept_any_certificate: false,
            connect_timeout: None,
            request_timeout: Self::TIMEOUT,
        }
    }

    // A probe that trusts only the certificates in `roots_pem`. The play zone's server
    // certificates chain to a root that is never added to a general store, so the
    // platform's roots would reject them and must not be accepted in its place.
    pub fn with_roots(roots_pem: &str) -> Result<Self, reqwest::Error> {
        Ok(Self {
            roots: Some(Certificate::from_pem_bundle(roots_pem.as_bytes())?),
            ..Self::new()
        })
    }

    // Certificate validation off. For reachability only.
    pub fn reachability(connect_timeout: Duration, request_timeout: Duration) -> Self {
        Self {
            roots: None,
            accept_any_certificate: true,
            connect_timeout: Some(connect_timeout),
            request_timeout,
        }
    }

    pub async fn serves_nonce(&self, name: &str, address: &str, port: u16, nonce: &str) -> bool {
        let Ok(ip) = address.parse::<IpAddr>() else {
            return false;
        };

        self.probe(name, ip, port, nonce).await == ProbeOutcome::Answered
    }

    pub async fn probe(&self, name: &str, address: IpAddr, port: u16, nonce: &str) -> ProbeOutcome {
        // `resolve` sends SNI and Host for the name, and dials the address. A redirect
        // would dial an address the caller chose, and a proxy would ignore `resolve`, so
        // neither is allowed.
        let mut builder = reqwest::Client::builder()
            .timeout(self.request_timeout)
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            .resolve(name, SocketAddr::new(address, port));
        if let Some(connect) = self.connect_timeout {
            builder = builder.connect_timeout(connect);
        }
        if self.accept_any_certificate {
            builder = builder.danger_accept_invalid_certs(true);
        }
        if let Some(roots) = &self.roots {
            builder = builder.tls_built_in_root_certs(false);
            for root in roots {
                builder = builder.add_root_certificate(root.clone());
            }
        }
        let client = match builder.build() {
            Ok(client) => client,
            Err(e) => {
                // A fault in the registry, not in the server's network. It is reported as
                // unreachable because no other outcome is closer.
                curia::error!("Address probe client could not be built", { "error": e.to_string() });
                return ProbeOutcome::TimedOut;
            }
        };

        let url = format!("https://{name}:{port}{}", Self::PATH);
        let mut response = match client.get(&url).send().await {
            Ok(response) => response,
            Err(e) => return Self::classify(&e),
        };

        if response.status().as_u16() != 200 {
            return ProbeOutcome::WrongService;
        }
        if response
            .content_length()
            .is_some_and(|length| length > Self::MAX_BODY as u64)
        {
            return ProbeOutcome::WrongService;
        }

        let mut body = Vec::new();
        loop {
            match response.chunk().await {
                Ok(Some(chunk)) => {
                    if body.len() + chunk.len() > Self::MAX_BODY {
                        return ProbeOutcome::WrongService;
                    }
                    body.extend_from_slice(&chunk);
                }
                Ok(None) => break,
                Err(e) => return Self::classify(&e),
            }
        }

        match std::str::from_utf8(&body) {
            Ok(text) if text.trim() == nonce => ProbeOutcome::Answered,
            _ => ProbeOutcome::WrongService,
        }
    }

    fn classify(error: &reqwest::Error) -> ProbeOutcome {
        if error.is_timeout() {
            return ProbeOutcome::TimedOut;
        }

        if error.is_connect() {
            match Self::connect_failure(error) {
                Some(ErrorKind::ConnectionRefused) => return ProbeOutcome::Refused,
                // A router or firewall answered that the host or network cannot be
                // reached (ICMP unreachable or prohibited, or no route). No service
                // answered, so this is a "could not reach" outcome, like a timeout.
                Some(ErrorKind::HostUnreachable | ErrorKind::NetworkUnreachable) => {
                    return ProbeOutcome::TimedOut;
                }
                _ => {}
            }
        }

        ProbeOutcome::WrongService
    }

    // Walks the source chain for the I/O error that failed the connection.
    fn connect_failure(error: &reqwest::Error) -> Option<ErrorKind> {
        let mut source: Option<&(dyn Error + 'static)> = error.source();
        while let Some(e) = source {
            if let Some(io) = e.downcast_ref::<std::io::Error>() {
                match io.kind() {
                    kind @ (ErrorKind::ConnectionRefused
                    | ErrorKind::HostUnreachable
                    | ErrorKind::NetworkUnreachable) => return Some(kind),
                    _ => {}
                }
            }
            source = e.source();
        }
        None
    }
}

impl Default for LiveAddressProbe {
    fn default() -> Self {
        Self::new()
    }
}

pub enum AddressProbe {
    Live(LiveAddressProbe),
    Fixed(bool),
    For(String),
    ForPort(String, u16, String),
}

impl AddressProbe {
    pub async fn serves_nonce(&self, name: &str, address: &str, port: u16, nonce: &str) -> bool {
        match self {
            Self::Live(probe) => probe.serves_nonce(name, address, port, nonce).await,
            Self::Fixed(answer) => *answer,
            Self::For(expected) => expected == name,
            Self::ForPort(expected, expected_port, expected_nonce) => {
                expected == name && *expected_port == port && expected_nonce == nonce
            }
        }
    }

    pub async fn probe(&self, name: &str, address: IpAddr, port: u16, nonce: &str) -> ProbeOutcome {
        match self {
            Self::Live(probe) => probe.probe(name, address, port, nonce).await,
            _ => {
                if self
                    .serves_nonce(name, &address.to_string(), port, nonce)
                    .await
                {
                    ProbeOutcome::Answered
                } else {
                    ProbeOutcome::WrongService
                }
            }
        }
    }
}
