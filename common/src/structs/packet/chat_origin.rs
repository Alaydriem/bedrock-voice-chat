use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Where a chat line came from.
///
/// `Bridge` is the seam for a future Discord relay: nothing in the chat sync feature produces
/// it, but its presence means a bridge can be added without a wire change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, TS)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub enum ChatOrigin {
    Game,
    App,
    Bridge,
}
