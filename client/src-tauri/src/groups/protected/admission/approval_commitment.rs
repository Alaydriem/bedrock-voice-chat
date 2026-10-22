use super::ApprovalTranscript;

/// The joiner's commitment to its KeyPackage and nonce, sent before it sees the member's nonce.
pub struct ApprovalCommitment;

impl ApprovalCommitment {
    const LABEL: &'static [u8] = b"bvc-join-commit-v1";

    /// `SHA-256("bvc-join-commit-v1" ‖ 0x00 ‖ lp16(gid) ‖ lp32(kp) ‖ n_j)`. `None` when a field is
    /// too long to length-prefix.
    pub fn of(gid: &str, key_package: &[u8], n_j: &[u8; 32]) -> Option<[u8; 32]> {
        Some(
            ApprovalTranscript::new(Self::LABEL)
                .short(&ApprovalTranscript::group_id(gid))?
                .long(key_package)?
                .fixed(n_j)
                .finish(),
        )
    }
}
