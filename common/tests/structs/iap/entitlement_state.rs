use common::structs::iap::EntitlementState;

#[test]
fn inactive_is_never_entitled() {
    let s = EntitlementState {
        active: false,
        paid_through: Some(i64::MAX),
    };
    assert!(!s.is_entitled_at(0));
}

#[test]
fn active_without_paid_through_is_entitled() {
    let s = EntitlementState {
        active: true,
        paid_through: None,
    };
    assert!(s.is_entitled_at(0));
    assert!(s.is_entitled_at(i64::MAX));
}

#[test]
fn active_before_expiry_is_entitled() {
    let s = EntitlementState {
        active: true,
        paid_through: Some(100),
    };
    assert!(s.is_entitled_at(99));
    assert!(!s.is_entitled_at(100));
    assert!(!s.is_entitled_at(101));
}
