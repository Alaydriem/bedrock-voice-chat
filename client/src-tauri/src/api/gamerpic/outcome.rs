use common::response::GamerpicResponse;

/// What a gamerpic lookup produced.
#[derive(Debug)]
pub(crate) enum GamerpicOutcome {
    /// The server's answer, which may carry no picture.
    Found(GamerpicResponse),
    /// The server refused this device's certificate on the lookup, and the probe that followed
    /// confirmed it no longer accepts it.
    CredentialsRejected,
}
