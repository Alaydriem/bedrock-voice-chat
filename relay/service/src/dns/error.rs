#[derive(Debug, thiserror::Error)]
pub enum DnsError {
    #[error("cloudflare request failed: {0}")]
    Http(String),
    #[error("cloudflare returned status {status}: {body}")]
    Status { status: u16, body: String },
    #[error("cloudflare returned no record id")]
    MissingRecordId,
    #[error("database: {0}")]
    Database(#[from] sea_orm::DbErr),
}
