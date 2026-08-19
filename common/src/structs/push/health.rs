use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::structs::network::ConnectionHealth;

/// Why the metrics stream is silent, or that it is not.
///
/// The snapshot stream only publishes while a session is connected, so a subscriber watching for
/// loss sees frames simply stop when the link fails — which is the moment it most needs an
/// answer. This frame names the failure.
///
/// A separate envelope rather than fields on the snapshot: a snapshot with zeroed measurements
/// renders as a flawless link with a 0 ms round trip, which misleads worse than no frame at all.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct HealthPush {
    #[serde(rename = "type")]
    pub kind: String,
    pub data: ConnectionHealth,
}

impl HealthPush {
    pub const KIND: &'static str = "health";

    pub fn new(data: ConnectionHealth) -> Self {
        Self {
            kind: Self::KIND.to_string(),
            data,
        }
    }
}
