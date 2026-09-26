use std::error::Error;

/// Renders an error together with its full `source()` chain.
///
/// A `reqwest::Error` displays as `error sending request for url (...)` whatever went wrong.
/// The cause that identifies the fault — a connect timeout, a TLS failure, a reset — is
/// reachable only through `source()`, so anything reporting the top-level Display alone
/// reports that a request failed and nothing about why.
pub struct ErrorChain;

impl ErrorChain {
    pub fn render(error: &dyn Error) -> String {
        let mut rendered = error.to_string();
        let mut source = error.source();

        while let Some(cause) = source {
            rendered.push_str(": ");
            rendered.push_str(&cause.to_string());
            source = cause.source();
        }

        rendered
    }
}
