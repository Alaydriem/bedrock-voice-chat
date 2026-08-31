#[derive(Debug, thiserror::Error)]
pub enum PairingError {
    #[error("the pairing store could not be reached: {0}")]
    Database(#[from] sea_orm::DbErr),

    #[error("a stored pairing row is unreadable: {0}")]
    Corrupt(String),
}
