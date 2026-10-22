/// Why a member's check did not produce a code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckError {
    /// A reveal for another check.
    WrongCheck,
    /// A reveal from a socket the `Check` did not go to. It does not use up the check.
    WrongSender,
    CommitmentMismatch,
    InvalidKeyPackage,
    /// A KeyPackage this group saw in the last 10 minutes.
    Replayed,
    /// The KeyPackage names someone other than the identity the server gave for the socket.
    IdentityMismatch,
    AlreadyRevealed,
    Expired,
}

impl CheckError {
    /// A failed check: someone may be interfering. The others are stray or late frames.
    pub fn is_failure(&self) -> bool {
        matches!(
            self,
            Self::CommitmentMismatch
                | Self::InvalidKeyPackage
                | Self::Replayed
                | Self::IdentityMismatch
        )
    }
}
