use bvc_client_lib::network::CredentialFault;

// QUIC carries a TLS alert as 0x100 + the alert number. 0x130 is unknown_ca, which is what a
// client sees when the CA it stored at login no longer signs what the server presents.
#[test]
fn every_certificate_alert_is_a_credential_fault() {
    for code in [0x12A, 0x12B, 0x12C, 0x12D, 0x12E, 0x130, 0x131, 0x174] {
        assert!(
            CredentialFault::is_credential_code(code),
            "0x{code:X} should be a credential fault"
        );
    }
}

// A cipher or version mismatch says nothing about this player's credentials, and wiping a
// keyring for one would be a worse failure than the one being fixed.
#[test]
fn other_tls_alerts_are_not_credential_faults() {
    for code in [0x128, 0x146, 0x14A] {
        assert!(
            !CredentialFault::is_credential_code(code),
            "0x{code:X} should not be a credential fault"
        );
    }
}

#[test]
fn codes_outside_the_alert_range_are_not_credential_faults() {
    for code in [0x0, 0x1, 0x9, 0xFF, 0x200, 0x1000] {
        assert!(
            !CredentialFault::is_credential_code(code),
            "0x{code:X} should not be a credential fault"
        );
    }
}

// The WebSocket transport reaches the same TLS alerts through rustls rather than through a QUIC
// code. A fault that de-authed over UDP and did not over TCP would make the de-auth depend on
// which transport happened to win the race.
#[test]
fn the_rustls_alert_set_matches_the_quic_one() {
    use rustls::AlertDescription;

    for alert in [
        AlertDescription::BadCertificate,
        AlertDescription::UnsupportedCertificate,
        AlertDescription::CertificateRevoked,
        AlertDescription::CertificateExpired,
        AlertDescription::CertificateUnknown,
        AlertDescription::UnknownCA,
        AlertDescription::AccessDenied,
        AlertDescription::CertificateRequired,
    ] {
        assert!(
            CredentialFault::is_credential_alert(alert),
            "{alert:?} should be a credential fault"
        );
    }

    for alert in [
        AlertDescription::HandshakeFailure,
        AlertDescription::ProtocolVersion,
        AlertDescription::InternalError,
    ] {
        assert!(
            !CredentialFault::is_credential_alert(alert),
            "{alert:?} should not be a credential fault"
        );
    }
}

// tokio-tungstenite buries the rustls error under its own error and an io::Error, so the
// classifier has to walk the chain rather than match the top-level type.
#[test]
fn a_rustls_error_is_found_through_a_wrapped_chain() {
    use rustls::{CertificateError, Error as TlsError};

    let buried = std::io::Error::other(TlsError::InvalidCertificate(
        CertificateError::UnknownIssuer,
    ));

    assert!(CredentialFault::in_tls_chain(&buried));
}

#[test]
fn an_unrelated_io_error_is_not_a_credential_fault() {
    let refused = std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "refused");

    assert!(!CredentialFault::in_tls_chain(&refused));
}
