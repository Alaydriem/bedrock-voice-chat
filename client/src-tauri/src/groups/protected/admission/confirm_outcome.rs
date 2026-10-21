/// What the joiner's "It matches" or "No match" did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmOutcome {
    Confirmed,
    /// The codes did not match. The request is over.
    Mismatch,
    /// The answer named a check this request is not in.
    Stale,
}
