use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;

use common::curia;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use super::address_probe::PlayAddressProbe;
use super::check_in::PlayCheckIn;
use super::error::PlayError;
use super::issuance_service::PlayIssuanceService;
use super::name_service::PlayNameService;
use super::preflight_service::PlayPreflightService;
use super::zone::PlayZone;

// Everything the enrollment endpoint needs for play names: claims, certificates, and the
// check-in that decides which address the zone serves.
pub struct PlayPlane {
    names: Arc<PlayNameService>,
    issuance: Arc<PlayIssuanceService>,
    zone: Arc<PlayZone>,
    probe: PlayAddressProbe,
    preflight: Arc<PlayPreflightService>,
    // Held across a database change and the zone update that follows it, so the zone
    // sees changes in the order their serials were issued. The zone drops a change whose
    // serial is behind its own, and two changes applied out of order would lose one.
    changes: Mutex<()>,
    // The probe in flight for each node, with a generation to tell it from its successor.
    // A new check-in cancels the earlier probe: one per node.
    probes: std::sync::Mutex<HashMap<String, (u64, CancellationToken)>>,
}

impl PlayPlane {
    pub fn new(
        names: Arc<PlayNameService>,
        issuance: Arc<PlayIssuanceService>,
        zone: Arc<PlayZone>,
        probe: PlayAddressProbe,
        preflight: Arc<PlayPreflightService>,
    ) -> Self {
        Self {
            names,
            issuance,
            zone,
            probe,
            preflight,
            changes: Mutex::new(()),
            probes: std::sync::Mutex::new(HashMap::new()),
        }
    }

    pub fn new_shared(
        names: Arc<PlayNameService>,
        issuance: Arc<PlayIssuanceService>,
        zone: Arc<PlayZone>,
        probe: PlayAddressProbe,
        preflight: Arc<PlayPreflightService>,
    ) -> Arc<Self> {
        Arc::new(Self::new(names, issuance, zone, probe, preflight))
    }

    pub fn preflight(&self) -> &Arc<PlayPreflightService> {
        &self.preflight
    }

    // The node's fully qualified play name.
    pub async fn claim(&self, node_id: &str, observed: IpAddr) -> Result<String, PlayError> {
        let label = self.names.claim(node_id, observed).await?;
        Ok(self.names.fqdn(&label))
    }

    pub async fn issue(&self, node_id: &str, csr_der: &[u8]) -> Result<String, PlayError> {
        self.issuance.issue(node_id, csr_der).await
    }

    pub async fn check_in(
        &self,
        node_id: &str,
        address: IpAddr,
        port: u16,
    ) -> Result<PlayCheckIn, PlayError> {
        self.names.record_check_in(node_id, address, port).await
    }

    // Probes the candidate address in the background and, on success, publishes it. A
    // changed address that never proves itself is withdrawn: leaving the old record would
    // point the name at an address nobody has checked since the server moved. A throttled
    // check-in starts nothing.
    pub fn verify_and_publish(
        self: &Arc<Self>,
        node_id: String,
        check_in: PlayCheckIn,
        address: IpAddr,
        port: u16,
    ) {
        if check_in.throttled {
            return;
        }

        let token = CancellationToken::new();
        let generation = self.begin_probe(&node_id, token.clone());
        let plane = Arc::clone(self);

        tokio::spawn(async move {
            let fqdn = plane.names.fqdn(&check_in.name);
            let proven = tokio::select! {
                _ = token.cancelled() => {
                    return;
                }
                proven = plane.probe.verify(&fqdn, address, port, &check_in.nonce) => proven,
            };

            let outcome = if proven {
                plane.publish(&node_id, &check_in.name, address, &check_in.nonce).await
            } else if check_in.address_changed {
                plane.withdraw(&node_id, &check_in.name, &check_in.nonce).await
            } else {
                Ok(())
            };
            plane.end_probe(&node_id, generation);

            if let Err(e) = outcome {
                curia::warn!("Play record update failed", { "name": check_in.name.clone(), "error": e.to_string() });
            }
        });
    }

    // Releases names with no recent check-in and takes them out of the zone.
    pub async fn release_stale(&self) -> Result<(), PlayError> {
        let _guard = self.changes.lock().await;

        let (released, serial) = self.names.release_stale().await?;
        for label in &released {
            self.zone.withdraw(label, serial);
        }

        if !released.is_empty() {
            curia::info!("Stale play names released", { "released": released.len().to_string() });
        }

        Ok(())
    }

    // Cancels the node's earlier probe and registers this one.
    fn begin_probe(&self, node_id: &str, token: CancellationToken) -> u64 {
        let mut probes = self.probes.lock().expect("the probe table is not poisoned");
        let generation = probes.get(node_id).map_or(0, |(g, _)| g + 1);
        if let Some((_, earlier)) = probes.insert(node_id.to_string(), (generation, token)) {
            earlier.cancel();
        }
        generation
    }

    fn end_probe(&self, node_id: &str, generation: u64) {
        let mut probes = self.probes.lock().expect("the probe table is not poisoned");
        if probes.get(node_id).is_some_and(|(g, _)| *g == generation) {
            probes.remove(node_id);
        }
    }

    async fn publish(
        &self,
        node_id: &str,
        label: &str,
        address: IpAddr,
        nonce: &str,
    ) -> Result<(), PlayError> {
        let _guard = self.changes.lock().await;

        // `None`: a newer check-in replaced this one while it was probing.
        if let Some(serial) = self.names.mark_published(node_id, address, nonce).await? {
            self.zone.publish(label, address, serial);
        }
        Ok(())
    }

    async fn withdraw(&self, node_id: &str, label: &str, nonce: &str) -> Result<(), PlayError> {
        let _guard = self.changes.lock().await;

        if let Some(serial) = self.names.mark_withdrawn(node_id, nonce).await? {
            self.zone.withdraw(label, serial);
        }
        Ok(())
    }
}
