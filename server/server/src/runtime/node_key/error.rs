/// A stored relay node key that cannot be used.
///
/// Both variants are fatal on purpose. A secret key is exactly 32 bytes, so anything else is
/// a corrupted value, and minting a replacement would change this node's public key — the
/// key every far-side `peer` block names.
#[derive(Debug, thiserror::Error)]
pub enum NodeKeyError {
    #[error("the stored relay node key is not valid hexadecimal")]
    NotHex,
    #[error("the relay node key is {0} bytes; a secret key is exactly 32")]
    WrongLength(usize),
}
