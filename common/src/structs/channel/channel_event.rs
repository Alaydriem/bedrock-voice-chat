use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::channel_events::ChannelEvents;

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct ChannelEvent {
    pub event: ChannelEvents,
}

impl ChannelEvent {
    pub fn new(event: ChannelEvents) -> Self {
        Self { event }
    }
}
