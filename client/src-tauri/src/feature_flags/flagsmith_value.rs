use crate::feature_flags::FeatureFlagService;

// How each value type is fetched from Flagsmith via OpenFeature. Adding a
// new kind of flag (string, JSON-shaped, …) is a single new impl here;
// every flag struct that uses the new type picks it up automatically via
// `FeatureFlag::Value`.
pub trait FlagsmithValue: Sized {
    // Fetch this flag's current value, returning `default` on miss / error.
    fn fetch(
        svc: &FeatureFlagService,
        key: &str,
        default: Self,
    ) -> impl std::future::Future<Output = Self> + Send;
}

impl FlagsmithValue for bool {
    async fn fetch(svc: &FeatureFlagService, key: &str, default: bool) -> bool {
        svc.lookup_bool(key).await.unwrap_or(default)
    }
}

impl FlagsmithValue for Option<i64> {
    async fn fetch(svc: &FeatureFlagService, key: &str, default: Option<i64>) -> Option<i64> {
        svc.get_int_value(key).await.or(default)
    }
}
