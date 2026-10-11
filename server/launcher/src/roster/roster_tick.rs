/// What one roster tick did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RosterTick {
    NotReady(String),
    NoEndpoint,
    NoAllowlist,
    NothingPending,
    Registered(usize),
    Rejected(String),
}
