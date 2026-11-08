use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// The operating system and hardware a mobile client runs on, in the shape of Sentry's `os` and
/// `device` contexts. Desktop clients have none: Sentry's own context integration already names
/// them correctly.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct DeviceInfo {
    /// `Android`, `iOS` or `iPadOS`.
    pub os_name: String,
    /// The marketing version: `16` on Android, `26.1` on iOS.
    pub os_version: Option<String>,
    /// The Android API level, or the iOS build (`23B85`).
    pub os_build: Option<String>,
    pub kernel_version: Option<String>,
    pub manufacturer: Option<String>,
    pub brand: Option<String>,
    /// The name a person would recognize where the platform reports one (`OnePlus 13`,
    /// `Pixel 9 Pro`). On iOS this is the hardware identifier (`iPhone17,3`), which Sentry
    /// resolves to the marketing name itself.
    pub model: Option<String>,
    /// The vendor's model code (`CPH2655`, `SM-S928B`), or the iOS board (`D47AP`).
    pub model_id: Option<String>,
    /// `iPhone` or `iPad`. Unset on Android, where Sentry has no equivalent.
    pub family: Option<String>,
}
