//! Folding a relay assignment into the configuration the rest of startup reads.

use crate::config::{Acme, AcmeProviderKind, ApplicationConfig};

// Two consumers read `tls.names`, and both must see the assigned name before they
// run: `CaCertManager`, which signs the local CA the QUIC listener presents as its
// own leaf, and `AcmeService`, which orders the public certificate.
//
// The certificate order list is set explicitly rather than left to the fallback.
// `effective_domains` falls back to the DNS entries of `tls.names`, which carries
// `localhost` by default — not parseable as an IP, so it survives the filter, enters
// the order, and fails the whole issuance.
pub struct EnrollmentStep;

impl EnrollmentStep {
    // Idempotent. A server re-reads its stored name on every boot, and a duplicate
    // entry in the SAN set would read to `SanKeySet` as drift and re-sign the CA on
    // every start.
    pub fn apply(config: &mut ApplicationConfig, name: String) {
        if !config.server.tls.names.contains(&name) {
            config.server.tls.names.push(name.clone());
        }

        config.server.tls.acme = Some(Acme {
            provider: Some(AcmeProviderKind::BvcRelay),
            domains: Some(vec![name]),
            ..Acme::default()
        });
    }
}
