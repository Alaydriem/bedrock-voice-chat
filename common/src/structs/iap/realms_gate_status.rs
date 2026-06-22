use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::gate_reason::GateReason;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum RealmsGateStatus {
    Allowed { reason: GateReason },
    FeatureDisabled,
    NotEntitled,
}
