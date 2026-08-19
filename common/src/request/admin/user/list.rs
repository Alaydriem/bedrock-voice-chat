use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::Game;

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct AdminUserListQuery {
    #[serde(default)]
    pub page: Option<u32>,
    #[serde(default)]
    pub page_size: Option<u32>,
    #[serde(default)]
    pub search: Option<String>,
    #[serde(default)]
    pub game: Option<Game>,
}
