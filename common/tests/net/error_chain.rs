use common::net::ErrorChain;
use std::error::Error;
use std::fmt;

#[derive(Debug)]
struct Layer {
    message: &'static str,
    cause: Option<Box<Layer>>,
}

impl fmt::Display for Layer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message)
    }
}

impl Error for Layer {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.cause
            .as_ref()
            .map(|cause| cause.as_ref() as &(dyn Error + 'static))
    }
}

// The defect this exists for: the outermost message is identical for every transport
// failure, so a report that stops there identifies nothing.
#[test]
fn the_cause_below_the_top_level_message_is_rendered() {
    let error = Layer {
        message: "error sending request for url (https://example.invalid/api/auth/minecraft)",
        cause: Some(Box::new(Layer {
            message: "operation timed out",
            cause: None,
        })),
    };

    assert_eq!(
        ErrorChain::render(&error),
        "error sending request for url (https://example.invalid/api/auth/minecraft): operation timed out"
    );
}

// A transport failure is routinely three deep: the request error, the connector error, and
// the operating system error that actually says what happened.
#[test]
fn a_chain_deeper_than_one_cause_is_walked_to_the_end() {
    let error = Layer {
        message: "outer",
        cause: Some(Box::new(Layer {
            message: "middle",
            cause: Some(Box::new(Layer {
                message: "inner",
                cause: None,
            })),
        })),
    };

    assert_eq!(ErrorChain::render(&error), "outer: middle: inner");
}

#[test]
fn an_error_with_no_cause_renders_as_itself() {
    let error = Layer {
        message: "alone",
        cause: None,
    };

    assert_eq!(ErrorChain::render(&error), "alone");
}
