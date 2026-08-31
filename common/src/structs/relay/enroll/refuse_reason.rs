use serde::{Deserialize, Serialize};

// Why the relay refused. Append-only: postcard encodes a variant as its index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EnrollRefuseReason {
    NoCommonVersion,
    UnknownToken,
    TokenAlreadyRedeemed,
    NotEntitled,
    AlreadyRegistered,
    NotRegistered,
    Suspended,
    NameNotOwned,
    Internal,
}
