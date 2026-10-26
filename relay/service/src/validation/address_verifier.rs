use std::sync::Arc;

use crate::dns::ZoneWriter;

use super::address_probe::AddressProbe;

pub struct AddressVerifier {
    probe: AddressProbe,
    zone: Arc<ZoneWriter>,
}

impl AddressVerifier {
    pub fn new(probe: AddressProbe, zone: Arc<ZoneWriter>) -> Self {
        Self { probe, zone }
    }

    pub fn new_shared(probe: AddressProbe, zone: Arc<ZoneWriter>) -> Arc<Self> {
        Arc::new(Self::new(probe, zone))
    }

    // The stored label is not a hostname, so the probe gets the fully qualified name.
    pub async fn serves_nonce(&self, label: &str, address: &str, port: u16, nonce: &str) -> bool {
        let fqdn = self.zone.address_fqdn(label);
        self.probe.serves_nonce(&fqdn, address, port, nonce).await
    }
}
