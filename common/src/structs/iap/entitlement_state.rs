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
