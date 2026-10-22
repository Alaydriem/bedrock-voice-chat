/// Why a joiner will not enter the group a Welcome describes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WelcomeRefusal {
    /// The joiner has not said the codes match. The Welcome is held, not discarded.
    NotConfirmed,
    /// Not the check this joiner revealed to, or not from the socket that sent it.
    WrongCheck,
    /// Signed by a key other than the one in the `Check`.
    WrongSigner,
    /// For an MLS group other than the one in the `Check`.
    WrongGroup,
}
