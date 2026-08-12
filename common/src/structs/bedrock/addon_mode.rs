use serde::{Deserialize, Serialize};
use ts_rs::TS;

// Who owns event delivery for a world: the world's own BVC addon over HTTP, or
// this client's proxy carrying events in-band. Declared by the operator per
// advertised server, because the client cannot observe it — addon liveness is
// keyed on the addon's own world uuid, and nothing maps that back to a
// configured server entry.
//
// This says nothing about how the client reaches the BVC server. That is decided
// independently by `TransportVerdict`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub enum AddonMode {
    // The addon posts to the BVC server itself. The proxy is a dumb relay:
    // in-band carriage would duplicate everything the addon already owns.
    #[default]
    Net,
    // No HTTP channel. The proxy carries positions, state and chat in-band.
    NoNet,
}

impl AddonMode {
    pub fn relays_only(&self) -> bool {
        matches!(self, Self::Net)
    }
}
