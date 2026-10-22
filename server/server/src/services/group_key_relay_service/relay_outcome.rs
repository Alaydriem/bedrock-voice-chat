use common::errors::ProtectedGroupError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RelayOutcome {
    Routed,
    /// A `Direct { to: Any }` with no announced member socket to receive it.
    NoAdmitter,
    Dropped(ProtectedGroupError),
}
