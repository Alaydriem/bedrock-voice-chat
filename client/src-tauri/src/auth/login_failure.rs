use common::net::ErrorChain;
use std::error::Error;

/// The message a failed login reports.
///
/// A transport failure displays as `error sending request for url (...)` whatever caused it,
/// so a message built from that alone says a request failed and nothing about why.
pub struct LoginFailure;

impl LoginFailure {
    pub fn describe(endpoint: &str, error: &dyn Error) -> String {
        format!(
            "Login request to {} failed: {}",
            endpoint,
            ErrorChain::render(error)
        )
    }
}
