use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct LinkJavaIdentityRequest {
    pub code: String,
    pub redirect_uri: String,
    pub client_id: String,
    pub gamertag: String,
}
