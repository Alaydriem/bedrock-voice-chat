use super::ApprovalTranscript;

/// The 8-digit code both people compare. Each side's app computes it from values the other side
/// committed to first, so a server in the middle matches both sides with probability 1 in 10⁸.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApprovalCode {
    value: u32,
}

impl ApprovalCode {
    const LABEL: &'static [u8] = b"bvc-join-code-v1";
    const MODULUS: u64 = 100_000_000;

    /// `u64_be(SHA-256("bvc-join-code-v1" ‖ 0x00 ‖ lp16(gid) ‖ lp32(kp) ‖ n_j ‖ n_a ‖
    /// lp16(signature_key) ‖ lp16(mls_group_id))[0..8]) mod 100_000_000`. `None` when a field is
    /// too long to length-prefix.
    pub fn derive(
        gid: &str,
        key_package: &[u8],
        n_j: &[u8; 32],
        n_a: &[u8; 32],
        signature_key: &[u8],
        mls_group_id: &[u8],
    ) -> Option<Self> {
        let digest = ApprovalTranscript::new(Self::LABEL)
            .short(&ApprovalTranscript::group_id(gid))?
            .long(key_package)?
            .fixed(n_j)
            .fixed(n_a)
            .short(signature_key)?
            .short(mls_group_id)?
            .finish();
        let mut head = [0u8; 8];
        head.copy_from_slice(&digest[..8]);
        let value = u64::from_be_bytes(head) % Self::MODULUS;
        Some(Self {
            value: value as u32,
        })
    }

    /// Eight digits, zero-padded.
    pub fn as_digits(&self) -> String {
        format!("{:08}", self.value)
    }

    /// Eight digits as two groups of four, the way both apps show them.
    pub fn display(&self) -> String {
        let digits = self.as_digits();
        format!("{} {}", &digits[..4], &digits[4..])
    }
}
