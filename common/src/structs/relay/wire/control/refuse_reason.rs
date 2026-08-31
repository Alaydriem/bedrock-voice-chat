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
    // The code is not one this server minted, or it was minted by a server whose
    // database has since been replaced.
    UnknownCode,
    // Already redeemed, or its attempt budget is spent.
    CodeSpent,
    CodeExpired,
}
