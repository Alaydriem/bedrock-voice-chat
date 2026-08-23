use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::Game;
use crate::structs::permission::Permission;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct IntrospectResponse {
    pub gamertag: String,
    pub game: Game,
    pub cert_not_after: Option<i64>,
    pub permissions: Vec<Permission>,
}
