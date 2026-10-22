use sha2::{Digest, Sha256};

/// The SHA-256 input for a commitment or a code: a domain label, a zero byte, then fields, each
/// length-prefixed so no two field sequences hash the same.
pub struct ApprovalTranscript {
    hasher: Sha256,
}

impl ApprovalTranscript {
    pub fn new(label: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(label);
        hasher.update([0u8]);
        Self { hasher }
    }

    /// A field behind a 2-byte big-endian length. `None` when it is too long to encode.
    pub fn short(mut self, field: &[u8]) -> Option<Self> {
        let length = u16::try_from(field.len()).ok()?;
        self.hasher.update(length.to_be_bytes());
        self.hasher.update(field);
        Some(self)
    }

    /// A field behind a 4-byte big-endian length. `None` when it is too long to encode.
    pub fn long(mut self, field: &[u8]) -> Option<Self> {
        let length = u32::try_from(field.len()).ok()?;
        self.hasher.update(length.to_be_bytes());
        self.hasher.update(field);
        Some(self)
    }

    /// A fixed-size field, which needs no length.
    pub fn fixed(mut self, field: &[u8]) -> Self {
        self.hasher.update(field);
        self
    }

    pub fn finish(self) -> [u8; 32] {
        self.hasher.finalize().into()
    }

    /// The group id as it enters every transcript: ASCII upper case, so the code does not
    /// depend on how a player typed it.
    pub fn group_id(gid: &str) -> Vec<u8> {
        gid.to_ascii_uppercase().into_bytes()
    }
}
