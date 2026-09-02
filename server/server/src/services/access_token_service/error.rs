#[derive(Debug, thiserror::Error)]
pub enum AccessTokenError {
    #[error("database error: {0}")]
    Database(#[from] sea_orm::DbErr),
    #[error("no token with id `{0}`")]
    UnknownId(String),
    #[error(
        "the legacy token comes from the environment or config.hcl; change it there, \
         because startup re-applies that value"
    )]
    LegacyIsConfigured,
    #[error("failed to reload access tokens: {0}")]
    Reload(String),
}
