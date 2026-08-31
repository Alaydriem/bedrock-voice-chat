use common::structs::relay::PairingCode;

// Crockford base32 excludes I, L, O and U so a code read off a screen cannot be
// mistyped into a different valid code. The normaliser folds the characters a person
// substitutes anyway.
#[test]
fn normalize_folds_the_characters_a_person_substitutes() {
    assert_eq!(PairingCode::normalize("k7m4-9qtr"), "K7M49QTR");
    assert_eq!(PairingCode::normalize(" K7M4 9QTR "), "K7M49QTR");
    assert_eq!(PairingCode::normalize("IK7M49QTR"), "1K7M49QTR");
    assert_eq!(PairingCode::normalize("lK7M49QTR"), "1K7M49QTR");
    assert_eq!(PairingCode::normalize("OK7M49QTR"), "0K7M49QTR");
}

#[test]
fn a_generated_code_verifies_against_its_own_digest() {
    let (plaintext, code) = PairingCode::generate();

    assert!(code.verify(&plaintext));
}

#[test]
fn a_generated_code_is_the_declared_length_and_alphabet() {
    let (plaintext, _) = PairingCode::generate();

    assert_eq!(plaintext.len(), PairingCode::PLAINTEXT_LEN);
    assert!(
        plaintext
            .bytes()
            .all(|b| b"0123456789ABCDEFGHJKMNPQRSTVWXYZ".contains(&b)),
        "got: {plaintext}"
    );
}

#[test]
fn generate_does_not_repeat_itself() {
    let (first, _) = PairingCode::generate();
    let (second, _) = PairingCode::generate();

    assert_ne!(first, second);
}

#[test]
fn verify_accepts_the_forms_a_person_types() {
    let (plaintext, code) = PairingCode::generate();
    let spaced = format!("{} {}", &plaintext[..4], &plaintext[4..]);
    let dashed = format!("{}-{}", &plaintext[..4], &plaintext[4..]);

    assert!(code.verify(&plaintext.to_lowercase()));
    assert!(code.verify(&spaced));
    assert!(code.verify(&dashed));
}

#[test]
fn verify_rejects_a_different_code() {
    let (_, code) = PairingCode::generate();
    let (other, _) = PairingCode::generate();

    assert!(!code.verify(&other));
}

#[test]
fn a_digest_stored_as_hex_still_verifies_the_original_code() {
    let code = PairingCode::from_plaintext("K7M49QTR");
    let stored = code.to_hex();

    let loaded = PairingCode::from_hex(&stored).expect("hex digest parses");

    assert!(loaded.verify("K7M49QTR"));
    assert!(!loaded.verify("K7M49QTS"));
}

#[test]
fn from_hex_rejects_a_value_that_is_not_a_digest() {
    assert!(PairingCode::from_hex("zzzz").is_err());
    assert!(PairingCode::from_hex("").is_err());
    assert!(PairingCode::from_hex(&"a".repeat(63)).is_err());
    assert!(PairingCode::from_hex(&"a".repeat(65)).is_err());
}
