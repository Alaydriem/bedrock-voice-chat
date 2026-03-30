use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::events::ChannelEvents;
use crate::Game;

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, TS)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct ChannelEvent {
    pub event: ChannelEvents,
    #[serde(default)]
    pub game: Option<Game>,
}

impl ChannelEvent {
    pub fn new(event: ChannelEvents) -> Self {
        Self { event, game: None }
    }
}
