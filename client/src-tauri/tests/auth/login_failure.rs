use bvc_client_lib::LoginFailure;
use std::error::Error;
use std::fmt;

#[derive(Debug)]
struct TimedOut;

impl fmt::Display for TimedOut {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("operation timed out")
    }
}

impl Error for TimedOut {}

#[derive(Debug)]
struct Transport;

impl fmt::Display for Transport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("error sending request for url (https://example.invalid/api/auth/minecraft)")
    }
}

impl Error for Transport {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&TimedOut)
    }
}

// The defect. Every report of this class read identically, because the part that says what
// happened sits below the message that was being logged.
#[test]
fn the_cause_below_the_transport_error_reaches_the_message() {
    let message = LoginFailure::describe("https://example.invalid/api/auth/minecraft", &Transport);

    assert!(
        message.contains("operation timed out"),
        "the cause is missing from: {}",
        message
    );
}

// Which request failed is the other half. A player reporting this reads it back to an
// operator, and "a request failed" identifies nothing to look at.
#[test]
fn the_endpoint_that_failed_is_named() {
    let message = LoginFailure::describe("https://example.invalid/api/auth/minecraft", &Transport);

    assert!(
        message.contains("https://example.invalid/api/auth/minecraft"),
        "the endpoint is missing from: {}",
        message
    );
}
