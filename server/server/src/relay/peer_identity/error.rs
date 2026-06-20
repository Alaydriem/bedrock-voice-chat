// Why a server-peer code redemption was refused.
#[derive(Debug, PartialEq, Eq)]
pub enum RedeemError {
    NotFound,
    Expired,
    AlreadyUsed,
    WrongRecipient,
    AtCapacity,
}

impl std::fmt::Display for RedeemError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RedeemError::NotFound => write!(f, "server-peer code not found"),
            RedeemError::Expired => write!(f, "server-peer code expired"),
            RedeemError::AlreadyUsed => write!(f, "server-peer code already used"),
            RedeemError::WrongRecipient => {
                write!(f, "server-peer code presented by the wrong recipient")
            }
            RedeemError::AtCapacity => write!(f, "server-peer identity store at capacity"),
        }
    }
}

impl std::error::Error for RedeemError {}
