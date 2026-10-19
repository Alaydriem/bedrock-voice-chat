use sha2::{Digest, Sha256};

/// A short code members compare out of band to confirm they share one group state.
///
/// Derived from the epoch authenticator, which every member of an epoch computes identically and
/// a forked group cannot match, so two members reading different codes are not in the same group.
pub struct SafetyCode {
    authenticator: Vec<u8>,
}

impl SafetyCode {
    const GROUPS: usize = 6;

    pub fn new(authenticator: &[u8]) -> Self {
        Self {
            authenticator: authenticator.to_vec(),
        }
    }

    /// Six groups of five digits, space-separated.
    pub fn digits(&self) -> String {
        let digest = Sha256::digest(&self.authenticator);
        digest
            .chunks_exact(4)
            .take(Self::GROUPS)
            .map(|chunk| {
                let value = u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                format!("{:05}", value % 100_000)
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
}
