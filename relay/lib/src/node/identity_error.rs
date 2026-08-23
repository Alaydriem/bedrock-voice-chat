use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum NodeIdentityError {
    #[error("reading the node key at {path}: {source}")]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("writing the node key to {path}: {source}")]
    Write {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("the node key at {path} is {len} bytes; a secret key is exactly 32")]
    Malformed { path: PathBuf, len: usize },
}
