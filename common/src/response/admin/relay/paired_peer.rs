use serde::{Deserialize, Serialize};

// One bridge that redeemed a pairing code.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
pub struct PairedPeer {
    pub node_id: String,
    pub label: String,
    pub paired_at: i64,
}
