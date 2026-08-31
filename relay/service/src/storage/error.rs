#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("the stored node key is {0} characters, not 64 hex digits")]
    MalformedNodeKey(usize),
    #[error("database: {0}")]
    Database(#[from] sea_orm::DbErr),
}
