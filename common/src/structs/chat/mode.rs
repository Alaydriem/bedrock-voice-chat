use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Which chat implementation is live for a world.
///
/// Distinct from availability: this says whether a mod or the local proxy is the source of
/// chat, not whether chat works right now. A world can be `Server` mode with its chat channel
/// down, and the composer has to tell those apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, TS)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub enum ChatMode {
    Server,
    Local,
}
