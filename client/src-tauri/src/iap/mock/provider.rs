use std::sync::atomic::{AtomicBool, Ordering};

use async_trait::async_trait;

use common::consts::iap::REALMS_PRODUCT_IDS;
use common::structs::iap::IapOffer;

use crate::iap::provider::EntitlementProvider;

// Local-preview entitlement source: returns canned offers and simulates
// purchase/restore in memory, so the Realms Connect upsell and gating can be
// exercised without store products configured. Constructed only in debug
// builds when `BVC_MOCK_IAP` is set (see `lib.rs`); never active in release.
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
        vec![
            IapOffer {
                product_id: REALMS_PRODUCT_IDS[0].to_string(),
                title: "Realms Connect — Annual".to_string(),
                description: "Proximity voice on every Realm you join. Billed yearly.".to_string(),
                formatted_price: Some("$14.99".to_string()),
            },
            IapOffer {
                product_id: REALMS_PRODUCT_IDS[1].to_string(),
                title: "Realms Connect — Monthly".to_string(),
                description: "Proximity voice on every Realm you join. Billed monthly.".to_string(),
                formatted_price: Some("$1.99".to_string()),
            },
        ]
    }

    async fn purchase(&self, _product_id: String) -> Result<bool, String> {
        self.entitled.store(true, Ordering::Relaxed);
        Ok(true)
    }

    async fn restore(&self) -> Result<bool, String> {
        Ok(self.is_entitled())
    }
}
