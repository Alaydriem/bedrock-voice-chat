#[derive(Debug, thiserror::Error)]
pub enum NamingError {
    #[error("the name space is exhausted; no unassigned name remains")]
    Exhausted,
    #[error("database: {0}")]
    Database(#[from] sea_orm::DbErr),
}
