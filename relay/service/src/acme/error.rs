#[derive(Debug, thiserror::Error)]
pub enum AcmeError {
    #[error("cloudflare request failed: {0}")]
    Http(String),
    #[error("no cloudflare zone found for {0}; the API token must reach that zone")]
    NoZone(String),
    #[error("the challenge TXT for {0} did not become publicly visible in time")]
    Propagation(String),
    #[error("acme: {0}")]
    Acme(String),
    #[error(transparent)]
    Storage(#[from] crate::storage::StorageError),
}
