use serde::{Deserialize, Serialize};

// Why a link was refused.
//
// Postcard encodes a variant as its index, so this list is append-only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RefuseReason {
    NoCommonVersion,
    NotAuthorized,
    AtCapacity,
    // The dialer declared no world the acceptor will carry. A link that
    // connects and carries nothing is indistinguishable from a healthy one
    // until someone notices the silence, so it is refused instead.
    NoSharedWorld,
}
