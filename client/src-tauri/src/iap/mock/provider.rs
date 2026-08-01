use std::sync::atomic::{AtomicBool, Ordering};

use async_trait::async_trait;

use common::structs::iap::IapOffer;

use crate::iap::provider::EntitlementProvider;

// Local-preview entitlement source: simulates purchase/restore in memory so a
// paid feature's flow can be exercised without store products configured.
// Constructed only in debug builds when `BVC_MOCK_IAP` is set (see `lib.rs`);
// never active in release.
pub struct MockProvider {
    entitled: AtomicBool,
}

impl MockProvider {
    pub fn new() -> Self {
        Self {
            entitled: AtomicBool::new(false),
        }
    }
}

#[async_trait]
impl EntitlementProvider for MockProvider {
    fn is_entitled(&self) -> bool {
        self.entitled.load(Ordering::Relaxed)
    }

    async fn check_and_refresh(&self) -> Result<bool, String> {
        Ok(self.is_entitled())
    }

    async fn offers(&self) -> Vec<IapOffer> {
        Vec::new()
    }

    async fn purchase(&self, _product_id: String) -> Result<bool, String> {
        self.entitled.store(true, Ordering::Relaxed);
        Ok(true)
    }

    async fn restore(&self) -> Result<bool, String> {
        Ok(self.is_entitled())
    }
}
