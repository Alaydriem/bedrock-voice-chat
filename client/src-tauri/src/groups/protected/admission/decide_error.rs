/// Why a member's Admit was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecideError {
    /// The decision named a check that is not live: closed, replaced, or never this member's.
    StaleCheck,
    /// No reveal arrived, so there is no code the member could have compared.
    NotRevealed,
    Expired,
}
