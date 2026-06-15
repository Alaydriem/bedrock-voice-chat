use std::sync::Arc;

use common::structs::relay::{PeerCertResponse, RelayEndpoint};

use crate::services::CertificateService;

use super::peer_cert_issue_error::PeerCertIssueError;
use super::presence_gate::PresenceGate;

// Issues the in-memory peer client cert that bootstraps a server↔server media
// link. This is the ACCEPTOR side: the initiator fetches a cert here (over its
// SPKI-pinned HTTPS channel) and then dials the acceptor's QUIC listener with it.
//
// Issuance is GATED on mutual presence proof: a cert is signed for the requesting
// peer's `host:port` identity ONLY when that peer is mutually presence-proven for
// the shared `hashed_world`. An unproven (or unknown-world) request is denied.
//
// The credential travels back as `(ca_pem, cert_pem, key_pem)` over the
// initiator's pinned server↔server HTTPS channel; it is never persisted and is
// scoped to one peer identity.
pub struct PeerCertIssuer {
    cert_service: Arc<CertificateService>,
    presence: Arc<dyn PresenceGate>,
    ca_pem: String,
}

impl PeerCertIssuer {
    pub fn new(
        cert_service: Arc<CertificateService>,
        presence: Arc<dyn PresenceGate>,
        ca_pem: String,
    ) -> Self {
        Self {
            cert_service,
            presence,
            ca_pem,
        }
    }

    pub fn new_shared(
        cert_service: Arc<CertificateService>,
        presence: Arc<dyn PresenceGate>,
        ca_pem: String,
    ) -> Arc<Self> {
        Arc::new(Self::new(cert_service, presence, ca_pem))
    }

    // Issues a peer cert for `host:port` scoped to `hashed_world`. Returns
    // `NotProven` when the peer is not mutually presence-proven for that world
    // (default deny), or `Signing` when issuance fails internally.
    pub fn issue(
        &self,
        host: &str,
        port: u16,
        hashed_world: &str,
    ) -> Result<PeerCertResponse, PeerCertIssueError> {
        let endpoint = RelayEndpoint {
            host: host.to_string(),
            port,
            primary: false,
        };
        if !self.presence.is_proven(&endpoint, hashed_world) {
            return Err(PeerCertIssueError::NotProven {
                host: host.to_string(),
                port,
                hashed_world: hashed_world.to_string(),
            });
        }

        let (cert, key) = self.cert_service.sign_peer_cert(host, port)?;
        Ok(PeerCertResponse {
            ca_pem: self.ca_pem.clone(),
            cert_pem: cert.pem(),
            key_pem: key.serialize_pem(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::ca_cert::CaCertManager;
    use crate::services::relay::presence::PresenceProver;
    use crate::services::relay::peer_manager::PeerManager;
    use std::fs;
    use std::time::Instant;
    use tempfile::TempDir;

    fn issuer_with(presence: Arc<dyn PresenceGate>) -> (PeerCertIssuer, TempDir) {
        let dir = TempDir::new().expect("tempdir");
        let path = dir.path().to_str().expect("utf-8 path");
        CaCertManager::new(path)
            .ensure(&[String::from("localhost")])
            .expect("CA generation");
        let ca_pem = fs::read_to_string(format!("{path}/ca.crt")).expect("read ca.crt");
        let cert_service =
            Arc::new(CertificateService::new(path).expect("CertificateService::new"));
        (PeerCertIssuer::new(cert_service, presence, ca_pem), dir)
    }

    #[test]
    fn issues_cert_for_proven_peer() {
        let prover = PresenceProver::new_shared();
        let peer = RelayEndpoint {
            host: "peer".into(),
            port: 5000,
            primary: false,
        };
        let key = PeerManager::endpoint_key(&peer);
        let now = Instant::now();
        // Complete the mutual proof for world W against this peer.
        let token = prover.new_challenge("W", now);
        prover.record_observed_from_peer(&key, &token, now);
        prover.record_echoed_to_peer(&key, "W");
        assert!(prover.is_mutually_proven(&peer, "W"));

        let (issuer, _dir) = issuer_with(prover);
        let resp = issuer.issue("peer", 5000, "W").expect("should issue");
        assert!(resp.cert_pem.contains("BEGIN CERTIFICATE"));
        assert!(resp.key_pem.contains("PRIVATE KEY"));
        assert!(resp.ca_pem.contains("BEGIN CERTIFICATE"));
    }

    #[test]
    fn denies_cert_for_unproven_peer() {
        let prover = PresenceProver::new_shared();
        let (issuer, _dir) = issuer_with(prover);
        let err = issuer.issue("attacker", 6666, "W").unwrap_err();
        assert!(err.to_string().contains("not mutually proven"));
    }

    // The issued identity is world-scoped: proven for W does not yield a cert
    // for a different world W2.
    #[test]
    fn cert_issuance_is_world_scoped() {
        let prover = PresenceProver::new_shared();
        let peer = RelayEndpoint {
            host: "peer".into(),
            port: 5000,
            primary: false,
        };
        let key = PeerManager::endpoint_key(&peer);
        let now = Instant::now();
        let token = prover.new_challenge("W", now);
        prover.record_observed_from_peer(&key, &token, now);
        prover.record_echoed_to_peer(&key, "W");

        let (issuer, _dir) = issuer_with(prover);
        assert!(issuer.issue("peer", 5000, "W").is_ok());
        assert!(issuer.issue("peer", 5000, "W2").is_err());
    }
}
