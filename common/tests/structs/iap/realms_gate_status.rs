use common::structs::iap::{GateReason, RealmsGateStatus};

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
