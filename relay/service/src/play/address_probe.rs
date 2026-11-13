use std::net::IpAddr;
use std::time::Duration;

use common::curia;

use crate::config::PlayCaConfig;
use crate::validation::{AddressProbe, LiveAddressProbe};

use super::error::PlayError;
use super::root_pin::PlayRootPin;

// Proves a play name's candidate address reaches the server that holds the name.
//
// Asks for the nonce the registry just issued, over TLS, with the certificate checked
// against the play root alone. Tries again after each delay, because the server starts
// its listener around the same moment it checks in.
pub struct PlayAddressProbe {
    probe: AddressProbe,
    delays: Vec<Duration>,
}

impl PlayAddressProbe {
    pub const PRODUCTION_DELAYS: [Duration; 3] = [
        Duration::from_secs(2),
        Duration::from_secs(10),
        Duration::from_secs(30),
    ];

    // `delays` has one entry per attempt: the wait before it.
    pub fn new(probe: AddressProbe, delays: Vec<Duration>) -> Self {
        Self { probe, delays }
    }

    // The deployed probe: the play root only, at the production delays.
    pub fn live(config: &PlayCaConfig) -> Result<Self, PlayError> {
        let root = std::fs::read_to_string(&config.root).map_err(|e| {
            PlayError::Signer(format!("reading the play root {}: {e}", config.root))
        })?;
        match PlayRootPin::is_pinned(&root) {
            Ok(true) => {}
            Ok(false) => {
                curia::error!("The play root is not a root that clients pin. Release clients will refuse every play name this registry publishes.", { "root": config.root.clone() });
            }
            Err(e) => {
                curia::error!("The play root could not be compared with the roots clients pin", { "root": config.root.clone(), "error": e.to_string() });
            }
        }
        let probe = LiveAddressProbe::with_roots(&root)
            .map_err(|e| PlayError::Signer(format!("the play root is unusable: {e}")))?;

        Ok(Self::new(
            AddressProbe::Live(probe),
            Self::PRODUCTION_DELAYS.to_vec(),
        ))
    }

    // True at the first attempt that succeeds.
    pub async fn verify(&self, fqdn: &str, address: IpAddr, port: u16, nonce: &str) -> bool {
        for delay in &self.delays {
            tokio::time::sleep(*delay).await;

            if self
                .probe
                .serves_nonce(fqdn, &address.to_string(), port, nonce)
                .await
            {
                return true;
            }
        }

        false
    }
}
