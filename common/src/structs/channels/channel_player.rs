use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::Game;

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct ChannelPlayer {
    pub name: String,
    pub game: Option<Game>,
    pub gamerpic: Option<String>,
}
