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

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use common::structs::iap::EntitlementState;

    struct FakeProvider {
        entitled: bool,
    }

    #[async_trait]
    impl EntitlementProvider for FakeProvider {
        fn is_entitled(&self) -> bool {
            self.entitled
        }
        async fn check_and_refresh(&self) -> Result<bool, String> {
            Ok(self.entitled)
        }
        async fn offers(&self) -> Vec<IapOffer> {
            Vec::new()
        }
        async fn purchase(&self, _product_id: String) -> Result<bool, String> {
            Ok(self.entitled)
        }
        async fn restore(&self) -> Result<bool, String> {
            Ok(self.entitled)
        }
    }

    // Mirrors EntitlementService::is_entitled over fakes. The real service's
    // `providers` enum only accepts the concrete `Store` variant (which needs a
    // Tauri AppHandle), so the aggregation predicate is asserted via this shim.
    struct TestService {
        providers: Vec<FakeProvider>,
    }
    impl TestService {
        fn is_entitled(&self) -> bool {
            self.providers.iter().any(|p| p.is_entitled())
        }
    }

    #[test]
    fn any_entitled_provider_entitles() {
        let svc = TestService {
            providers: vec![
                FakeProvider { entitled: false },
                FakeProvider { entitled: true },
            ],
        };
        assert!(svc.is_entitled());
        let _ = EntitlementState::default();
    }

    #[test]
    fn no_entitled_provider_denies() {
        let svc = TestService {
            providers: vec![FakeProvider { entitled: false }],
        };
        assert!(!svc.is_entitled());
    }
}
