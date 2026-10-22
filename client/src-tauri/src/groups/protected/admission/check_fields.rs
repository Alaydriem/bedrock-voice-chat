use bytes::Bytes;

/// What a member's `Check` carries to the joiner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckFields {
    pub check_id: [u8; 16],
    pub nonce: [u8; 32],
    pub signature_key: Bytes,
    pub mls_group_id: Bytes,
    /// The checking member's canonical identity, as the member's app states it.
    pub member: String,
}
