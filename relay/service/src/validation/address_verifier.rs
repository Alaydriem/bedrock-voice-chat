use std::sync::Arc;

use base64::Engine;

use crate::dns::ZoneWriter;
use crate::enroll::EnrollSessions;

use super::address_probe::AddressProbe;

// The two proofs the daily pass asks of a registration: that the node holds its key,
// and that its declared address answers for its assigned name.
pub struct AddressVerifier {
    sessions: Arc<EnrollSessions>,
    probe: AddressProbe,
    zone: Arc<ZoneWriter>,
}

impl AddressVerifier {
    const NONCE_BYTES: usize = 32;

    pub fn new(sessions: Arc<EnrollSessions>, probe: AddressProbe, zone: Arc<ZoneWriter>) -> Self {
        Self {
            sessions,
            probe,
            zone,
        }
    }

    pub fn new_shared(
        sessions: Arc<EnrollSessions>,
        probe: AddressProbe,
        zone: Arc<ZoneWriter>,
    ) -> Arc<Self> {
        Arc::new(Self::new(sessions, probe, zone))
    }

    // Sends a fresh nonce down the node's held session and returns it when the node
    // signed it with its own key. `None` is a failed identity check.
    pub async fn challenge(&self, node_id: &str) -> Option<String> {
        let key = node_id.parse::<iroh::PublicKey>().ok()?;
        let nonce = Self::mint_nonce();

        let signature = self.sessions.challenge(&key, nonce.as_bytes()).await?;
        let bytes = <[u8; 64]>::try_from(signature.as_slice()).ok()?;

        key.verify(nonce.as_bytes(), &iroh::Signature::from_bytes(&bytes))
            .ok()?;
        Some(nonce)
    }

    // Whether `address` serves `nonce` under the assigned name.
    //
    // The probe is given the fully qualified name. The stored label is not a hostname,
    // and a certificate is never issued for it, so a probe against the label would
    // fail validation on every server and suspend them all.
    pub async fn serves_nonce(&self, label: &str, address: &str, nonce: &str) -> bool {
        let fqdn = self.zone.address_fqdn(label);
        self.probe.serves_nonce(&fqdn, address, nonce).await
    }

    fn mint_nonce() -> String {
        let mut bytes = [0u8; Self::NONCE_BYTES];
        getrandom::fill(&mut bytes).expect("the system random source is available");
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
    }
}
