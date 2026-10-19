/// What a hostile relay does on top of the server's normal rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hostility {
    None,
    /// Replaces a joiner's KeyPackage with its own after the PAKE, keeping the joiner's MAC.
    SwapKeyPackage,
    /// Answers a joiner's offer itself with a Welcome into a group the relay runs.
    InjectWelcome,
    /// Re-delivers the first accepted commit to everyone after later commits.
    ReplayFirstCommit,
    /// Accepts two commits for one epoch, delivering each to a different half of the group.
    ForkOnNextCommit,
    /// Answers every PAKE init itself, with a guessed password.
    PakeToSelf,
}
