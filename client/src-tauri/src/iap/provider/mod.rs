use async_trait::async_trait;
use enum_dispatch::enum_dispatch;

use common::structs::iap::IapOffer;

<<<<<<< HEAD
use crate::iap::mock::MockProvider;
=======
>>>>>>> 86597dc (chore: initial iap variant)
use crate::iap::store::StoreProvider;

// Platform entitlement interface; `EntitlementService` aggregates one or more.
// `is_entitled` is a synchronous read of cached state (never blocks on a store
// round-trip); the async methods refresh that cache or drive a store flow.
#[async_trait]
#[enum_dispatch]
pub trait EntitlementProvider {
    fn is_entitled(&self) -> bool;

    async fn check_and_refresh(&self) -> Result<bool, String>;

    async fn offers(&self) -> Vec<IapOffer>;

    async fn purchase(&self, product_id: String) -> Result<bool, String>;

    async fn restore(&self) -> Result<bool, String>;
}

#[enum_dispatch(EntitlementProvider)]
pub enum EntitlementProviderType {
    Store(StoreProvider),
    Mock(MockProvider),
}
