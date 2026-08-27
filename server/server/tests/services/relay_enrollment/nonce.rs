use bvc_server_lib::services::relay_enrollment::{CurrentNonce, EnrollmentError};
use common::structs::relay::enroll::EnrollRefuseReason;

// Before the relay has sent a challenge there is nothing to echo. The HTTP route
// reports that as absent rather than as an empty value, which would look to the relay
// like a node answering with the wrong nonce.
#[test]
fn a_nonce_that_was_never_set_is_absent() {
    assert_eq!(CurrentNonce::new_shared().get(), None);
}

#[test]
fn the_most_recent_nonce_replaces_the_previous_one() {
    let nonce = CurrentNonce::new_shared();
    nonce.set("first".to_string());

    nonce.set("second".to_string());

    assert_eq!(nonce.get(), Some("second".to_string()));
}

// A refusal reaches the operator as a sentence telling them what to do, not as a
// variant name. This is the only place a wire code becomes something readable, and it
// is what an operator sees when enrollment fails at first boot.
#[test]
fn every_refusal_reason_explains_itself() {
    for reason in [
        EnrollRefuseReason::NoCommonVersion,
        EnrollRefuseReason::UnknownToken,
        EnrollRefuseReason::TokenAlreadyRedeemed,
        EnrollRefuseReason::NotEntitled,
        EnrollRefuseReason::AlreadyRegistered,
        EnrollRefuseReason::NotRegistered,
        EnrollRefuseReason::Suspended,
        EnrollRefuseReason::NameNotOwned,
        EnrollRefuseReason::Internal,
    ] {
        let message = EnrollmentError::refused(reason).to_string();

        assert!(
            message.len() > "the relay refused this server: ".len() + 20,
            "{reason:?} must explain itself, got {message:?}"
        );
        assert!(
            !message.contains("{"),
            "{reason:?} left a formatting placeholder in {message:?}"
        );
    }
}

// A spent token is the most common failure at first boot — an operator restarting a
// container that already enrolled. It has to say so plainly rather than reading as a
// bad token.
#[test]
fn a_spent_token_is_distinguished_from_an_unknown_one() {
    let spent = EnrollmentError::refused(EnrollRefuseReason::TokenAlreadyRedeemed).to_string();
    let unknown = EnrollmentError::refused(EnrollRefuseReason::UnknownToken).to_string();

    assert_ne!(spent, unknown);
    assert!(spent.contains("already been used"));
}
