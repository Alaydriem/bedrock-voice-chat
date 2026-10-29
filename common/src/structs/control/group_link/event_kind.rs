use serde::{Deserialize, Serialize};

/// What happened to a non-protected BVC channel.
///
/// `Protected` is the last event a channel ever produces: the bridge unlinks it, and nothing
/// about a protected channel reaches the bridge after.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
pub enum GroupLinkEventKind {
    Create,
    Join,
    Leave,
    Delete,
    Protected,
}
