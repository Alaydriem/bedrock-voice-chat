#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AdmissionError {
    /// Also what a joiner sees for a MAC that does not verify: it is never told which side failed.
    #[error("wrong password")]
    WrongPassword,
    #[error("a group password needs at least 6 characters")]
    PasswordTooShort,
    #[error("admission protocol error: {0}")]
    Protocol(String),
}
