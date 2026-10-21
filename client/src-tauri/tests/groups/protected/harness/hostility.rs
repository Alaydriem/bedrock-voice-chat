/// What a hostile relay does on top of the server's normal rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hostility {
    None,
    /// Replaces the KeyPackage in a joiner's reveal with one of its own, keeping the nonce.
    SwapKeyPackage,
    /// Replaces the nonce in a member's check with one of its own.
    SwapMemberNonce,
    /// Drops every reveal, so no check ever gets an answer.
    AbortReveals,
    /// Re-delivers the first accepted commit to everyone after later commits.
    ReplayFirstCommit,
    /// Accepts two commits for one epoch, delivering each to a different half of the group.
    ForkOnNextCommit,
}
