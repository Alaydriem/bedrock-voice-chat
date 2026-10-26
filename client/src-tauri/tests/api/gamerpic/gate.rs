use bvc_client_lib::GamerpicGate;

// Only the caller that closes the gate runs the credential probe; every concurrent 401 after it
// must see that someone already did.
#[test]
fn only_the_first_refusal_reports_the_flip() {
    let gate = GamerpicGate::new();

    assert!(gate.refuse());
    assert!(!gate.refuse());
    assert!(gate.is_refused());
}

// A 401 is PlayerGuard's MissingCertificate: no certificate reached the server, which is what a
// TLS-terminating proxy produces while voice on the same certificate works. A certificate the
// server distrusts fails the TLS handshake instead, so a 401 can never justify a sign-out.
#[test]
fn a_missing_certificate_is_not_probed() {
    assert!(!GamerpicGate::refusal_needs_probe(401));
}

// A 403 is the server refusing a certificate it did see: revoked, banished, or no such player.
#[test]
fn a_refused_certificate_is_probed() {
    assert!(GamerpicGate::refusal_needs_probe(403));
}

#[test]
fn a_fresh_gate_is_open() {
    assert!(!GamerpicGate::new().is_refused());
}

#[test]
fn a_miss_is_scoped_to_game_and_gamertag() {
    let gate = GamerpicGate::new();
    gate.record_miss("minecraft", "Steve");

    assert!(gate.is_miss("minecraft", "Steve"));
    assert!(!gate.is_miss("minecraft", "Alex"));
    assert!(!gate.is_miss("hytale", "Steve"));
}

// One gate per pooled server client: a server that refuses this device must not silence
// gamerpics from a server that accepts it.
#[test]
fn gates_are_independent() {
    let a = GamerpicGate::new();
    let b = GamerpicGate::new();

    a.refuse();
    a.record_miss("minecraft", "Steve");

    assert!(!b.is_refused());
    assert!(!b.is_miss("minecraft", "Steve"));
}
