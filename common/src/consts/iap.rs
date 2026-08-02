// Empty: no products are currently offered. Any ID added here must also be
// defined in every store console (App Store Connect, Play Console, Partner
// Centre) under this exact string.
pub const PRODUCT_IDS: [&str; 0] = [];

// tauri-plugin-iap product type for auto-renewing subscriptions.
pub const PRODUCT_TYPE_SUBS: &str = "subs";

// Keyring namespace and key for the cached entitlement snapshot.
pub const IAP_KEYRING_NS: &str = "iap";
pub const IAP_KEYRING_KEY_ENTITLEMENT: &str = "entitlement";
