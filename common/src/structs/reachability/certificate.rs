use serde::{Deserialize, Serialize};
use ts_rs::TS;

// What a server presented during a probe handshake. Reported, never trusted: the
// probe's verifier validates nothing, so every field here describes what was
// observed rather than what was accepted.
//
// The input arrives from the network, so every failure mode returns `None` rather
// than panicking.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct ObservedCertificate {
    pub subject: String,
    pub issuer: String,
    pub not_after: i64,
    pub sans: Vec<String>,
}

impl ObservedCertificate {
    pub fn from_der(der: &[u8]) -> Option<Self> {
        let (_, cert) = x509_parser::parse_x509_certificate(der).ok()?;

        let sans = cert
            .subject_alternative_name()
            .ok()
            .flatten()
            .map(|ext| {
                ext.value
                    .general_names
                    .iter()
                    .map(|name| name.to_string())
                    .collect()
            })
            .unwrap_or_default();

        Some(Self {
            subject: cert.subject().to_string(),
            issuer: cert.issuer().to_string(),
            not_after: cert.validity().not_after.timestamp(),
            sans,
        })
    }
}
