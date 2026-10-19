#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseOutcome {
    MembersRemain,
    /// The last socket closed. The group's key state existed only on member devices, so the
    /// group has ended and its channel must go with it.
    GroupEmpty,
}
