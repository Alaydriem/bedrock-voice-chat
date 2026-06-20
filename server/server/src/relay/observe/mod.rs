pub mod handler;

pub use handler::ProductionObservedCodeHandler;

use async_trait::async_trait;
use common::structs::relay::{PeerCertResponse, RelayEndpoint};

// Handles a `!bvcp` code a local client observed in the realm (the asker side of
// Flow 1). The QUIC input path calls this when a `PeerPresenceObserved` packet
// arrives; the production impl redeems the code against the minter and opens the
// world-bound peer link. Implementations must be non-blocking.
pub trait ObservedCodeHandler: Send + Sync {
    fn on_observed(&self, token: String);
}

// Decrypts an observed (sealed, hex-encoded) realm token to the plaintext code.
// Abstracts the sealed-box unseal so the observe handler's selection logic is
// testable without real crypto. Returns `None` when the token is not ours.
pub trait CodeDecryptor: Send + Sync {
    fn decrypt(&self, observed: &str) -> Option<String>;
}

// Redeems an observed code at a minter for an in-memory server-peer credential.
// Abstracts `RelayClient::peer_redeem` so the observe handler's minter-selection
// logic is testable without HTTP.
#[async_trait]
pub trait CodeRedeemer: Send + Sync {
    async fn redeem(
        &self,
        minter_host: &str,
        minter_http_port: u16,
        code: &str,
        presenter: &RelayEndpoint,
    ) -> Result<PeerCertResponse, anyhow::Error>;
}
