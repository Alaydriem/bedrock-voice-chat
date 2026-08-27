use common::structs::relay::enroll::EnrollRefuseReason;

#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    #[error("this Discord account does not hold a qualifying membership")]
    NotEntitled,
    #[error("this Discord account already holds an assigned name")]
    AlreadyRegistered,
    #[error("unknown enrollment token")]
    UnknownToken,
    #[error("this enrollment token has already been redeemed")]
    TokenAlreadyRedeemed,
    #[error("no registration for this node")]
    NotRegistered,
    #[error("this registration is suspended")]
    Suspended,
    #[error("this node does not own {0}")]
    NameNotOwned(String),
    #[error(transparent)]
    Naming(#[from] crate::naming::NamingError),
    #[error(transparent)]
    Discord(#[from] crate::discord::DiscordError),
    #[error("database: {0}")]
    Database(#[from] sea_orm::DbErr),
}

impl RegistryError {
    // What the far side is told. Deliberately coarser than the local error: an
    // enrolling node learns why it was refused, not what else the registry knows.
    pub fn refuse_reason(&self) -> EnrollRefuseReason {
        match self {
            Self::NotEntitled => EnrollRefuseReason::NotEntitled,
            Self::AlreadyRegistered => EnrollRefuseReason::AlreadyRegistered,
            Self::UnknownToken => EnrollRefuseReason::UnknownToken,
            Self::TokenAlreadyRedeemed => EnrollRefuseReason::TokenAlreadyRedeemed,
            Self::NotRegistered => EnrollRefuseReason::NotRegistered,
            Self::Suspended => EnrollRefuseReason::Suspended,
            Self::NameNotOwned(_) => EnrollRefuseReason::NameNotOwned,
            _ => EnrollRefuseReason::Internal,
        }
    }
}
