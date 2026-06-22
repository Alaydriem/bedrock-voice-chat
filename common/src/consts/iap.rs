// Canonical subscription product IDs. These exact strings must be defined in
// every store console (App Store Connect, Play Console, Partner Center).
pub const REALMS_PRODUCT_IDS: [&str; 2] = ["realms_connect_annual", "realms_connect_monthly"];

// tauri-plugin-iap product type for auto-renewing subscriptions.
pub const PRODUCT_TYPE_SUBS: &str = "subs";
