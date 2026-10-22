/// What a group action did, as the control plane reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GroupOutcome {
    /// Applied on the server. Carries the share code of a group it created.
    Done(Option<String>),
    /// A protected join, handed to the actor's own app to run admission.
    Forwarded,
    /// A protected join for an actor with no live app session.
    NoSession,
}
