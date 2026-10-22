use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Whether the server supports end-to-end encrypted groups.
///
/// A server that predates this field sends nothing and reads as unsupported, so the app hides the
/// option rather than offering a create that would 404.
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct ApiConfigProtectedGroups {
    pub enabled: bool,
}
