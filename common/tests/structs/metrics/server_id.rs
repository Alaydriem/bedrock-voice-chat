use common::structs::metrics::ServerId;

const PEM: &str = "-----BEGIN CERTIFICATE-----\nMIIB...\n-----END CERTIFICATE-----";

#[test]
fn the_same_ca_yields_the_same_id() {
    assert_eq!(ServerId::from_ca_pem(PEM.as_bytes()), ServerId::from_ca_pem(PEM.as_bytes()));
}

#[test]
fn a_different_ca_yields_a_different_id() {
    let other = PEM.replace("MIIB", "MIIC");
    assert_ne!(
        ServerId::from_ca_pem(PEM.as_bytes()),
        ServerId::from_ca_pem(other.as_bytes())
    );
}

#[test]
fn trailing_newlines_do_not_change_the_id() {
    // The failure this guards is silent: a server reads the CA from disk while a client holds it
    // as a string from its credential store. One trailing newline between them would produce a
    // different hash, the analytics join would never close, and nothing would report an error.
    let from_disk = format!("{PEM}\n");
    let from_store = PEM.to_string();
    let with_crlf = format!("{PEM}\r\n");

    let expected = ServerId::from_ca_pem(from_store.as_bytes());
    assert_eq!(ServerId::from_ca_pem(from_disk.as_bytes()), expected);
    assert_eq!(ServerId::from_ca_pem(with_crlf.as_bytes()), expected);
}

#[test]
fn interior_whitespace_still_matters() {
    // Only trailing whitespace is insignificant. Two genuinely different certificates that happen
    // to differ mid-body must not collide.
    let altered = PEM.replace("MIIB...", "MIIB ...");
    assert_ne!(
        ServerId::from_ca_pem(PEM.as_bytes()),
        ServerId::from_ca_pem(altered.as_bytes())
    );
}

#[test]
fn an_empty_ca_does_not_panic() {
    assert!(!ServerId::from_ca_pem(b"").is_empty());
    assert!(!ServerId::from_ca_pem(b"\n\n  \n").is_empty());
}
