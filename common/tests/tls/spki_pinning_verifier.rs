use base64::Engine;
use common::tls::SpkiPinningVerifier;

#[test]
fn decodes_valid_pins_and_skips_garbage() {
    let valid = base64::engine::general_purpose::STANDARD.encode([0u8; 32]);
    let verifier = SpkiPinningVerifier::new(&[valid, "not-base64-32".to_string()]);
    assert!(verifier.has_pins());
    assert_eq!(verifier.pin_count(), 1);
}

#[test]
fn empty_pins_reports_no_pins() {
    let verifier = SpkiPinningVerifier::new(&[]);
    assert!(!verifier.has_pins());
    assert_eq!(verifier.pin_count(), 0);
}
