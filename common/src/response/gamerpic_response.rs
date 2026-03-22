use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::Game;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct GamerpicResponse {
    pub gamertag: String,
    pub game: Game,
    pub gamerpic: Option<String>,
}
