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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allowed_serializes_with_status_and_reason() {
        let json = serde_json::to_string(&RealmsGateStatus::Allowed {
            reason: GateReason::FreeWeekend,
        })
        .unwrap();
        assert_eq!(json, r#"{"status":"allowed","reason":"free_weekend"}"#);
    }

    #[test]
    fn not_entitled_serializes_with_status_only() {
        let json = serde_json::to_string(&RealmsGateStatus::NotEntitled).unwrap();
        assert_eq!(json, r#"{"status":"not_entitled"}"#);
    }
}
