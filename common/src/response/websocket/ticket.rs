use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
pub struct WebsocketTicketResponse {
    pub ticket: String,
    /// Seconds until expiry, so the caller can decide whether to reuse a held
    /// ticket or fetch a fresh one before reconnecting.
    pub expires_in: u64,
}
