use serde::{Deserialize, Serialize};

// Asker → minter: redeem a peer code observed through the realm. The presenter
// endpoint must match the code's bound recipient (single-use, recipient-bound).
// On success the minter returns the in-memory peer cert (`PeerCertResponse`).
#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
pub struct PeerRedeemRequest {
    pub code: String,
    pub presenter_host: String,
    pub presenter_port: u16,
}
