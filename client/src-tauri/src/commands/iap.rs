use std::sync::Arc;

use tauri::State;

use common::structs::iap::IapOffer;
use common::structs::{AnalyticsEvent, AnalyticsEventData};

use crate::analytics::AnalyticsService;
use crate::iap::EntitlementService;

#[tauri::command(async)]
pub(crate) async fn iap_list_offers(
    entitlement: State<'_, Arc<EntitlementService>>,
) -> Result<Vec<IapOffer>, String> {
    Ok(entitlement.list_offers().await)
}

#[tauri::command(async)]
pub(crate) async fn iap_purchase(
    product_id: String,
    entitlement: State<'_, Arc<EntitlementService>>,
    analytics: State<'_, Arc<AnalyticsService>>,
) -> Result<bool, String> {
    analytics.track(
        AnalyticsEvent::RealmsPurchaseStarted,
        Some(AnalyticsEventData::new().insert("product_id", product_id.clone())),
    );
    match entitlement.purchase(product_id).await {
        Ok(v) => {
            analytics.track(AnalyticsEvent::RealmsPurchaseCompleted, None);
            Ok(v)
        }
        Err(e) => {
            analytics.track(
                AnalyticsEvent::RealmsPurchaseFailed,
                Some(AnalyticsEventData::new().insert("error", e.clone())),
            );
            Err(e)
        }
    }
}

#[tauri::command(async)]
pub(crate) async fn iap_restore(
    entitlement: State<'_, Arc<EntitlementService>>,
    analytics: State<'_, Arc<AnalyticsService>>,
) -> Result<bool, String> {
    analytics.track(AnalyticsEvent::RealmsRestoreInvoked, None);
    entitlement.restore().await
}

#[tauri::command(async)]
pub(crate) async fn iap_refresh(
    entitlement: State<'_, Arc<EntitlementService>>,
) -> Result<bool, String> {
    entitlement.check_and_refresh().await
}
