use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct EntitlementState {
    pub active: bool,
    // Epoch milliseconds (matches tauri-plugin-iap `expiration_time`).
    pub paid_through: Option<i64>,
}

impl EntitlementState {
    // `active` is the store's authoritative "owned + purchased" signal;
    // `paid_through` only bounds the offline cache. An active entitlement with
    // no known expiry is entitled (some platforms omit `expiration_time` for an
    // active license); a cached active entitlement past its expiry is not.
    pub fn is_entitled_at(&self, now: i64) -> bool {
        self.active && self.paid_through.is_none_or(|t| now < t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        // An active license the store reports without an expiry is entitled
        // (the cache has no horizon to enforce).
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
}
