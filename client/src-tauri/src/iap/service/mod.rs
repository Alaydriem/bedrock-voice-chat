use common::structs::iap::IapOffer;

use crate::iap::provider::{EntitlementProvider, EntitlementProviderType};

// Aggregates entitlement across providers: entitled iff ANY provider is.
pub struct EntitlementService {
    providers: Vec<EntitlementProviderType>,
}

impl EntitlementService {
    pub fn new(providers: Vec<EntitlementProviderType>) -> Self {
        Self { providers }
    }

    pub fn is_entitled(&self) -> bool {
        self.providers.iter().any(|p| p.is_entitled())
    }

    pub async fn check_and_refresh(&self) -> Result<bool, String> {
        for p in &self.providers {
            if let Err(e) = p.check_and_refresh().await {
                log::warn!("entitlement refresh failed for a provider: {e}");
            }
        }
        Ok(self.is_entitled())
    }

    pub async fn list_offers(&self) -> Vec<IapOffer> {
        let mut offers = Vec::new();
        for p in &self.providers {
            offers.extend(p.offers().await);
        }
        offers
    }

    pub async fn purchase(&self, product_id: String) -> Result<bool, String> {
        let mut last_err = "No purchasable entitlement source available.".to_string();
        for p in &self.providers {
            match p.purchase(product_id.clone()).await {
                Ok(v) => return Ok(v),
                Err(e) => last_err = e,
            }
        }
        Err(last_err)
    }

    pub async fn restore(&self) -> Result<bool, String> {
        let mut last_err = "No restorable entitlement source available.".to_string();
        for p in &self.providers {
            match p.restore().await {
                Ok(v) => return Ok(v),
                Err(e) => last_err = e,
            }
        }
        Err(last_err)
    }
}
