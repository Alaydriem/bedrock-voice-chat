use common::s2n_quic::connection::Error as ConnectionError;

/// Whether a voice handshake failed because this client's stored credentials cannot work.
///
/// QUIC carries a TLS alert as a transport error code of `0x100 + alert`. Read from the code
/// rather than from the rendered message: the message is a `Debug` string from a dependency,
/// and matching on it would make a de-authentication depend on s2n's formatting.
///
/// `initiator: Local` on the reported failure means this client rejected the server's chain —
/// the CA stored in the keyring at login no longer signs what the server presents. Nothing on
/// the device can repair that, which is why it ends in a logout rather than a retry.
pub struct CredentialFault;

impl CredentialFault {
    /// TLS alerts that name a certificate or the trust in it.
    ///
    /// Deliberately not the whole alert range. `handshake_failure` and `protocol_version` are
    /// statements about the negotiation, not about this player's identity.
    const CODES: [u64; 8] = [
        // bad_certificate
        0x12A,
        // unsupported_certificate
        0x12B,
        // certificate_revoked
        0x12C,
        // certificate_expired
        0x12D,
        // certificate_unknown
        0x12E,
        // unknown_ca
        0x130,
        // access_denied
        0x131,
        // certificate_required
        0x174,
    ];

    pub fn is_credential_code(code: u64) -> bool {
        Self::CODES.contains(&code)
    }

    pub fn in_connection(error: &ConnectionError) -> bool {
        match error {
            ConnectionError::Transport { code, .. } => Self::is_credential_code(code.as_u64()),
            _ => false,
        }
    }

    /// The same eight alerts as `CODES`, named the way rustls names them.
    ///
    /// The WebSocket transport reaches these through TLS directly rather than through a QUIC
    /// transport code. Kept deliberately identical: a fault that signs a player out over UDP
    /// and does not over TCP would make the outcome depend on which transport won the race.
    pub fn is_credential_alert(alert: rustls::AlertDescription) -> bool {
        matches!(
            alert,
            rustls::AlertDescription::BadCertificate
                | rustls::AlertDescription::UnsupportedCertificate
                | rustls::AlertDescription::CertificateRevoked
                | rustls::AlertDescription::CertificateExpired
                | rustls::AlertDescription::CertificateUnknown
                | rustls::AlertDescription::UnknownCA
                | rustls::AlertDescription::AccessDenied
                | rustls::AlertDescription::CertificateRequired
        )
    }

    /// Whether anything in an error's `source()` chain is a certificate failure.
    ///
    /// Walks the chain rather than matching the top-level type: tokio-tungstenite reports a TLS
    /// failure as its own error wrapping an `io::Error` wrapping the `rustls::Error`, and the
    /// depth is not part of any stable contract.
    ///
    /// `InvalidCertificate` is this client rejecting the server's chain; `AlertReceived` is the
    /// server rejecting this client's certificate. Both mean the credentials and the server have
    /// diverged.
    pub fn in_tls_chain(error: &(dyn std::error::Error + 'static)) -> bool {
        let mut current = Some(error);

        while let Some(cause) = current {
            if let Some(tls) = cause.downcast_ref::<rustls::Error>() {
                return match tls {
                    rustls::Error::InvalidCertificate(_) => true,
                    rustls::Error::AlertReceived(alert) => Self::is_credential_alert(*alert),
                    _ => false,
                };
            }

            // `io::Error::source` reports the source of the error it carries, not that error
            // itself, so a `rustls::Error` boxed into an `io::Error` is invisible to a plain
            // `source()` walk — and that is exactly how tokio-rustls delivers one. `get_ref` is
            // the only way back to it.
            if let Some(inner) = cause
                .downcast_ref::<std::io::Error>()
                .and_then(|io| io.get_ref())
            {
                current = Some(inner);
                continue;
            }

            current = cause.source();
        }

        false
    }
}
