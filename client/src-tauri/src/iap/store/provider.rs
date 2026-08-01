use async_trait::async_trait;
use parking_lot::RwLock;
use tauri::AppHandle;
use tauri_plugin_iap::{
    IapExt, Product, PurchaseRequest, PurchaseStateValue, RestorePurchasesRequest,
};

use common::consts::iap::{IAP_KEYRING_KEY_ENTITLEMENT, PRODUCT_IDS, PRODUCT_TYPE_SUBS};
use common::structs::iap::{EntitlementState, IapOffer};

use crate::iap::IapKeyringService;
use crate::iap::provider::EntitlementProvider;

// Store-backed entitlement via tauri-plugin-iap. Holds a cached
// `EntitlementState` (sourced from the keyring at construction, refreshed from
// the store on demand) so `is_entitled` is a non-blocking read.
pub struct StoreProvider {
    app_handle: AppHandle,
    cache: RwLock<EntitlementState>,
}

impl StoreProvider {
    pub fn new(app_handle: AppHandle) -> Self {
        let cache = RwLock::new(Self::load_cached(&app_handle));
        Self { app_handle, cache }
    }

    // Epoch milliseconds — matches tauri-plugin-iap `expiration_time`, which is
    // reported in ms across all platforms.
    fn now_millis() -> i64 {
        chrono::Utc::now().timestamp_millis()
    }

    fn load_cached(app_handle: &AppHandle) -> EntitlementState {
        match IapKeyringService::new(app_handle).load(IAP_KEYRING_KEY_ENTITLEMENT) {
            Some(json) => serde_json::from_str(&json).unwrap_or_default(),
            None => EntitlementState::default(),
        }
    }

    fn persist(&self, state: &EntitlementState) {
        if let Ok(json) = serde_json::to_string(state) {
            IapKeyringService::new(&self.app_handle).store(IAP_KEYRING_KEY_ENTITLEMENT, &json);
        }
    }

    // The only function that queries tauri-plugin-iap for entitlement. Returns
    // the freshest (active, paid_through) the store reports across
    // `PRODUCT_IDS`. `paid_through` is the max `expiration_time` (epoch ms).
    async fn query_store(&self) -> Result<EntitlementState, String> {
        // A false negative here would report a completed purchase as
        // not-entitled, so an unconfigured catalog is an error, not `inactive`.
        if PRODUCT_IDS.is_empty() {
            return Err("no IAP products are configured".to_string());
        }

        let mut active = false;
        let mut paid_through: Option<i64> = None;

        for product_id in PRODUCT_IDS {
            let status = self
                .app_handle
                .iap()
                .get_product_status(product_id.to_string(), PRODUCT_TYPE_SUBS.to_string())
                .await
                .map_err(|e| e.to_string())?;

            let purchased = status.is_owned
                && matches!(status.purchase_state, Some(PurchaseStateValue::Purchased));
            if purchased {
                active = true;
                if let Some(exp) = status.expiration_time {
                    paid_through = Some(paid_through.map_or(exp, |cur| cur.max(exp)));
                }
            }
        }

        Ok(EntitlementState {
            active,
            paid_through,
        })
    }

    // Android leaves the top-level `formatted_price` unset for subscriptions and
    // reports the price only under `subscription_offer_details`; iOS, macOS, and
    // Windows populate the top-level field directly. Prefer the top-level price
    // and fall back to the recurring (last) pricing phase of the first offer so
    // a subscription's price is not blank on any platform.
    pub fn display_price(product: &Product) -> Option<String> {
        if let Some(price) = &product.formatted_price {
            return Some(price.clone());
        }
        product
            .subscription_offer_details
            .as_ref()
            .and_then(|offers| offers.first())
            .and_then(|offer| offer.pricing_phases.last())
            .map(|phase| phase.formatted_price.clone())
    }
}

#[async_trait]
impl EntitlementProvider for StoreProvider {
    fn is_entitled(&self) -> bool {
        self.cache.read().is_entitled_at(Self::now_millis())
    }

    async fn check_and_refresh(&self) -> Result<bool, String> {
        let fresh = self.query_store().await?;
        // Writing the credential store is not free.
        if *self.cache.read() != fresh {
            self.persist(&fresh);
            *self.cache.write() = fresh;
        }
        Ok(self.is_entitled())
    }

    async fn offers(&self) -> Vec<IapOffer> {
        let products = match self
            .app_handle
            .iap()
            .get_products(
                PRODUCT_IDS.iter().map(|s| s.to_string()).collect(),
                PRODUCT_TYPE_SUBS.to_string(),
            )
            .await
        {
            Ok(resp) => resp.products,
            Err(e) => {
                log::warn!("iap get_products failed: {e}");
                return Vec::new();
            }
        };
        products
            .into_iter()
            .map(|p| {
                let formatted_price = Self::display_price(&p);
                IapOffer {
                    product_id: p.product_id,
                    title: p.title,
                    description: p.description,
                    formatted_price,
                }
            })
            .collect()
    }

    async fn purchase(&self, product_id: String) -> Result<bool, String> {
        let req = PurchaseRequest {
            product_id,
            product_type: PRODUCT_TYPE_SUBS.to_string(),
            options: None,
        };
        let purchase = self
            .app_handle
            .iap()
            .purchase(req)
            .await
            .map_err(|e| e.to_string())?;

        // Android auto-refunds purchases left unacknowledged > 3 days.
        // No-op where the platform auto-acknowledges (iOS/macOS/Windows).
        if !purchase.is_acknowledged {
            if let Err(e) = self
                .app_handle
                .iap()
                .acknowledge_purchase(purchase.purchase_token.clone())
                .await
            {
                log::warn!("iap acknowledge_purchase failed: {e}");
            }
        }

        self.check_and_refresh().await
    }

    async fn restore(&self) -> Result<bool, String> {
        let req = RestorePurchasesRequest {
            product_type: PRODUCT_TYPE_SUBS.to_string(),
            service_ticket: None,
            publisher_user_id: None,
        };
        self.app_handle
            .iap()
            .restore_purchases(req)
            .await
            .map_err(|e| e.to_string())?;
        self.check_and_refresh().await
    }
}
