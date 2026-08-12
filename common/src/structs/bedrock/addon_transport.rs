use serde::{Deserialize, Serialize};
use ts_rs::TS;

// How a world's BVC addon reaches the BVC server. Declared per advertised
// server by the operator, because the client cannot observe it: addon health is
// keyed on the addon's own world UUID, while the proxy only knows the id it
// derives from StartGame, and the two namespaces do not correlate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub enum AddonTransport {
    // No HTTP channel. The proxy carries positions and state in-band.
    #[default]
    NoNet,
    // The addon posts to the BVC server directly. In-band carriage is
    // redundant, and the addon cancels the chat rides before any peer sees them.
    Net,
}

impl AddonTransport {
    pub fn suppresses_position_feed(&self) -> bool {
        matches!(self, Self::Net)
    }

    pub fn suppresses_in_band_rides(&self) -> bool {
        matches!(self, Self::Net)
    }
}
