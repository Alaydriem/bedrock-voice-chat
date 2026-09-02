/// The verifiable part of one issued credential, held in memory.
#[derive(Debug, Clone)]
pub struct CachedToken {
    pub secret_hash: String,
    pub revoked_at: Option<i64>,
}
