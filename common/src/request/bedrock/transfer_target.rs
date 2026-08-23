use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct TransferTargetRequest {
    // No subject field. The target is keyed on the caller's own identity, taken from their
    // client certificate, so there is nothing here to forge — the route previously trusted a
    // caller-supplied xuid and would write a target for anybody.
    pub host: String,
    pub port: u16,
}
