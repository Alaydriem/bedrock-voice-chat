use std::time::{Duration, Instant};

use bvc_client_lib::groups::protected::{
    AdmissionRateLimiter, AdmitterSession, GroupPassword, JoinerSession,
};
use bytes::Bytes;
use common::errors::AdmissionError;
use common::structs::group_key::AdmissionMessage;

const GROUP: &str = "ABCD2345";
const KEY_PACKAGE: &[u8] = b"joiner key package";
const WELCOME: &[u8] = b"mls welcome";

fn password(raw: &str) -> GroupPassword {
    GroupPassword::new(raw).expect("valid password")
}

fn identity(canonical: &str) -> common::PlayerIdentity {
    canonical.parse().expect("canonical identity")
}

/// Runs the whole exchange and returns what each side ends with.
fn admit(
    joiner_password: &GroupPassword,
    admitter_password: &GroupPassword,
) -> Result<Vec<u8>, AdmissionError> {
    let (joiner, init) = JoinerSession::start(GROUP, joiner_password);
    let (admitter, reply) = AdmitterSession::respond(GROUP, admitter_password, init)?;
    let (awaiting, offer) = joiner.on_reply(reply, KEY_PACKAGE)?;
    let key_package = admitter.verify_offer(offer)?;
    assert_eq!(key_package, KEY_PACKAGE);
    let welcome = admitter.seal_welcome(WELCOME);
    awaiting.on_welcome(welcome)
}

#[test]
fn matching_passwords_admit_and_the_welcome_verifies() {
    let welcome = admit(&password("correct horse"), &password("correct horse")).unwrap();

    assert_eq!(welcome, WELCOME);
}

#[test]
fn wrong_password_fails_the_joiner_mac_at_the_admitter() {
    let result = admit(&password("guessing"), &password("correct horse"));

    assert_eq!(result.unwrap_err(), AdmissionError::WrongPassword);
}

// The server lets the real joiner pass the PAKE, then swaps in its own KeyPackage.
#[test]
fn key_package_swapped_after_pake_is_refused() {
    let pw = password("correct horse");
    let (joiner, init) = JoinerSession::start(GROUP, &pw);
    let (admitter, reply) = AdmitterSession::respond(GROUP, &pw, init).unwrap();
    let (_awaiting, offer) = joiner.on_reply(reply, KEY_PACKAGE).unwrap();

    let AdmissionMessage::KeyPackageOffer { mac, .. } = offer else {
        panic!("expected a key package offer");
    };
    let swapped = AdmissionMessage::KeyPackageOffer {
        key_package: Bytes::from_static(b"server key package"),
        mac,
    };

    assert_eq!(admitter.verify_offer(swapped).unwrap_err(), AdmissionError::WrongPassword);
}

// The server runs its own group and hands the joiner a Welcome into it.
#[test]
fn welcome_substituted_by_relay_is_refused() {
    let pw = password("correct horse");
    let (joiner, init) = JoinerSession::start(GROUP, &pw);
    let (admitter, reply) = AdmitterSession::respond(GROUP, &pw, init).unwrap();
    let (awaiting, _offer) = joiner.on_reply(reply, KEY_PACKAGE).unwrap();

    let genuine = admitter.seal_welcome(WELCOME);
    let AdmissionMessage::WelcomeOffer { mac, .. } = genuine else {
        panic!("expected a welcome offer");
    };
    let swapped = AdmissionMessage::WelcomeOffer {
        welcome: Bytes::from_static(b"server group welcome"),
        mac,
    };

    assert_eq!(awaiting.on_welcome(swapped).unwrap_err(), AdmissionError::WrongPassword);
}

// A relay that guesses the password runs its own admitter; its MAC must not verify.
#[test]
fn welcome_from_an_admitter_with_a_different_key_is_refused() {
    let (joiner, init) = JoinerSession::start(GROUP, &password("correct horse"));
    let (impostor, reply) = AdmitterSession::respond(GROUP, &password("a guess!"), init).unwrap();
    let (awaiting, _offer) = joiner.on_reply(reply, KEY_PACKAGE).unwrap();

    let forged = impostor.seal_welcome(WELCOME);

    assert_eq!(awaiting.on_welcome(forged).unwrap_err(), AdmissionError::WrongPassword);
}

#[test]
fn password_under_6_characters_is_rejected() {
    assert_eq!(GroupPassword::new("12345").unwrap_err(), AdmissionError::PasswordTooShort);
    assert!(GroupPassword::new("123456").is_ok());
}

#[test]
fn nfc_and_nfd_forms_of_the_same_password_admit() {
    let composed = password("caf\u{00e9} au lait");
    let decomposed = password("cafe\u{0301} au lait");

    assert!(admit(&composed, &decomposed).is_ok());
}

#[test]
fn trailing_space_is_significant() {
    let result = admit(&password("correct horse "), &password("correct horse"));

    assert_eq!(result.unwrap_err(), AdmissionError::WrongPassword);
}

// Two joiners land on the same admitter at once; neither session may leak into the other.
#[test]
fn two_concurrent_joiner_sessions_on_one_admitter_both_succeed() {
    let pw = password("correct horse");
    let (joiner_a, init_a) = JoinerSession::start(GROUP, &pw);
    let (joiner_b, init_b) = JoinerSession::start(GROUP, &pw);
    let (admitter_a, reply_a) = AdmitterSession::respond(GROUP, &pw, init_a).unwrap();
    let (admitter_b, reply_b) = AdmitterSession::respond(GROUP, &pw, init_b).unwrap();

    let (awaiting_b, offer_b) = joiner_b.on_reply(reply_b, b"kp-b").unwrap();
    let (awaiting_a, offer_a) = joiner_a.on_reply(reply_a, b"kp-a").unwrap();

    assert_eq!(admitter_a.verify_offer(offer_a).unwrap(), b"kp-a");
    assert_eq!(admitter_b.verify_offer(offer_b).unwrap(), b"kp-b");
    assert_eq!(awaiting_a.on_welcome(admitter_a.seal_welcome(b"w-a")).unwrap(), b"w-a");
    assert_eq!(awaiting_b.on_welcome(admitter_b.seal_welcome(b"w-b")).unwrap(), b"w-b");
}

#[test]
fn a_refusal_reads_as_wrong_password_to_the_joiner() {
    let pw = password("correct horse");
    let (joiner, init) = JoinerSession::start(GROUP, &pw);
    let (_admitter, reply) = AdmitterSession::respond(GROUP, &pw, init).unwrap();
    let (awaiting, _offer) = joiner.on_reply(reply, KEY_PACKAGE).unwrap();

    assert_eq!(
        awaiting.on_welcome(AdmissionMessage::Refused).unwrap_err(),
        AdmissionError::WrongPassword
    );
}

#[test]
fn rate_limiter_refuses_the_6th_failure_per_joiner_within_10_minutes() {
    let mut limiter = AdmissionRateLimiter::new();
    let joiner = identity("minecraft:Mallory");
    let start = Instant::now();

    for minute in 0..5u64 {
        let now = start + Duration::from_secs(minute * 60);
        assert!(limiter.permits(&joiner, now));
        limiter.record_failure(&joiner, now);
    }

    assert!(!limiter.permits(&joiner, start + Duration::from_secs(9 * 60)));
    assert!(limiter.permits(&identity("minecraft:Alice"), start + Duration::from_secs(9 * 60)));
}

#[test]
fn rate_limiter_refuses_the_21st_failure_per_group() {
    let mut limiter = AdmissionRateLimiter::new();
    let now = Instant::now();

    for n in 0..20 {
        let joiner = identity(&format!("minecraft:Guesser{n}"));
        assert!(limiter.permits(&joiner, now));
        limiter.record_failure(&joiner, now);
    }

    assert!(!limiter.permits(&identity("minecraft:Fresh"), now));
}

#[test]
fn rate_limiter_forgets_failures_after_10_minutes() {
    let mut limiter = AdmissionRateLimiter::new();
    let joiner = identity("minecraft:Mallory");
    let start = Instant::now();
    for _ in 0..5 {
        limiter.record_failure(&joiner, start);
    }

    assert!(!limiter.permits(&joiner, start + Duration::from_secs(599)));
    assert!(limiter.permits(&joiner, start + Duration::from_secs(601)));
}
