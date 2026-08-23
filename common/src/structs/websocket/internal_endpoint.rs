use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Where this process's own push channel is listening, and the credential for it.
///
/// Fetched by the webview with a single `invoke` at startup. `invoke` rather than an event on
/// purpose: a lost event listener registration is the fault this whole channel exists to escape,
/// so the channel must not be discovered through one.
///
/// The token is generated per process and never persisted. Loopback is not isolated between
/// applications on Android, so any app holding `INTERNET` can reach the port; the token is what
/// stops it reading who is speaking.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct InternalEndpoint {
    pub port: u16,
    pub token: String,
}
