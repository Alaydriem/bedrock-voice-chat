/// A uniform async key/value cache interface. Each concrete cache (owned solely by
/// `CacheManager`) implements this over its own key/value types, so callers get the
/// same `get`/`set`/`delete` shape everywhere. Concrete caches may add domain
/// helpers (e.g. scoped or owner-wide operations) beyond this base.
pub trait CacheTrait {
    type Key;
    type Value;

    async fn get(&self, key: &Self::Key) -> Option<Self::Value>;
    async fn set(&self, key: Self::Key, value: Self::Value);
    async fn delete(&self, key: &Self::Key);
}
