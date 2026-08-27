use std::net::SocketAddr;
use std::time::Duration;

// Fetches the challenge nonce from the address an operator declared.
//
// This is what binds a published address record to the node that asked for it.
// Without it, an operator can declare an address they do not control, pass the
// identity half forever because their node is genuinely fine, and leave the relay
// fronting a third party from its own zone.
pub struct LiveAddressProbe;

impl LiveAddressProbe {
    const TIMEOUT: Duration = Duration::from_secs(10);
    const HTTPS_PORT: u16 = 443;
    const PATH: &'static str = "/health/enrollment-nonce";

    pub fn new() -> Self {
        Self
    }

    pub async fn serves_nonce(&self, name: &str, address: &str, nonce: &str) -> bool {
        let Ok(ip) = address.parse::<std::net::IpAddr>() else {
            return false;
        };
        let resolved = SocketAddr::new(ip, Self::HTTPS_PORT);

        // `resolve` overrides the address while leaving the server name and Host
        // header alone, so the certificate is still validated against the assigned
        // name rather than against an address. A probe that skipped validation would
        // confirm only that something answers there.
        let Ok(client) = reqwest::Client::builder()
            .timeout(Self::TIMEOUT)
            .resolve(name, resolved)
            .build()
        else {
            return false;
        };

        let url = format!("https://{name}{}", Self::PATH);
        match client.get(&url).send().await {
            Ok(response) => response
                .text()
                .await
                .map(|body| body.trim() == nonce)
                .unwrap_or(false),
            Err(_) => false,
        }
    }
}

impl Default for LiveAddressProbe {
    fn default() -> Self {
        Self::new()
    }
}

// Enum delegation rather than a trait object, matching how the relay dispatches its
// other outbound dependencies.
pub enum AddressProbe {
    Live(LiveAddressProbe),
    Fixed(bool),
}

impl AddressProbe {
    pub async fn serves_nonce(&self, name: &str, address: &str, nonce: &str) -> bool {
        match self {
            Self::Live(probe) => probe.serves_nonce(name, address, nonce).await,
            Self::Fixed(answer) => *answer,
        }
    }
}
