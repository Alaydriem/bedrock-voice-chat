use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct WebsocketTicketResponse {
    pub ticket: String,
    /// Seconds until expiry, so the caller can decide whether to reuse a held
    /// ticket or fetch a fresh one before reconnecting.
    pub expires_in: u64,
}
