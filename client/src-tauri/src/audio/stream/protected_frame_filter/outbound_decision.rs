/// What to do with one encoded frame before it leaves for the server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutboundDecision {
    /// Not in a protected group: send the Opus as it is.
    Plain(Vec<u8>),
    /// In a protected group: send this SFrame ciphertext.
    Sealed(Vec<u8>),
    /// In a protected group without keys: send nothing.
    Drop,
}
