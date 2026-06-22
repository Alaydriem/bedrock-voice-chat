use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use common::structs::relay::{PeerCertResponse, RelayEndpoint};

use crate::relay::discovery::client::RelayClient;
use crate::relay::peer::dial::driver::RedeemedDial;
use crate::relay::peer::manager::PeerManager;
use crate::relay::peer_identity::ServerPeerStore;

use super::{CodeDecryptor, CodeRedeemer, ObservedCodeHandler};

#[async_trait]
impl CodeRedeemer for RelayClient {
    async fn redeem(
        &self,
        minter_host: &str,
        minter_http_port: u16,
        code: &str,
        presenter: &RelayEndpoint,
    ) -> Result<PeerCertResponse, anyhow::Error> {
        self.peer_redeem(minter_host, minter_http_port, code, presenter)
            .await
    }
}

// Production observe handler (asker side of Flow 1). For an observed code it
// tries each minter this server has an outstanding offer to: only the minter
// that issued the code accepts the redemption (recipient-bound, single-use). On
// success it authorizes that minter for the shared world (so the gate relays
// both directions) and dials it with the redeemed `server::`-CN
// credential, binding a world-scoped initiator link.
pub struct ProductionObservedCodeHandler {
    peer_manager: Arc<PeerManager>,
    server_peer_store: Arc<ServerPeerStore>,
    decryptor: Arc<dyn CodeDecryptor>,
    redeemer: Arc<dyn CodeRedeemer>,
    dial: Arc<dyn RedeemedDial>,
    self_endpoint: RelayEndpoint,
}

impl ProductionObservedCodeHandler {
    pub fn new(
        peer_manager: Arc<PeerManager>,
        server_peer_store: Arc<ServerPeerStore>,
        decryptor: Arc<dyn CodeDecryptor>,
        redeemer: Arc<dyn CodeRedeemer>,
        dial: Arc<dyn RedeemedDial>,
        self_endpoint: RelayEndpoint,
    ) -> Self {
        Self {
            peer_manager,
            server_peer_store,
            decryptor,
            redeemer,
            dial,
            self_endpoint,
        }
    }

    pub fn new_shared(
        peer_manager: Arc<PeerManager>,
        server_peer_store: Arc<ServerPeerStore>,
        decryptor: Arc<dyn CodeDecryptor>,
        redeemer: Arc<dyn CodeRedeemer>,
        dial: Arc<dyn RedeemedDial>,
        self_endpoint: RelayEndpoint,
    ) -> Arc<Self> {
        Arc::new(Self::new(
            peer_manager,
            server_peer_store,
            decryptor,
            redeemer,
            dial,
            self_endpoint,
        ))
    }

    // Splits a `host:{https_port}` endpoint key.
    fn split_endpoint(peer_ep: &str) -> Option<(String, u16)> {
        let (h, p) = peer_ep.rsplit_once(':')?;
        let port = p.parse::<u16>().ok()?;
        Some((h.to_string(), port))
    }

    // Tries the observed code against every minter we have an outstanding offer
    // to, stopping at the first that accepts it. Authorizes that minter for the
    // shared world and dials it with the redeemed credential.
    async fn redeem_and_link(
        peer_manager: Arc<PeerManager>,
        server_peer_store: Arc<ServerPeerStore>,
        decryptor: Arc<dyn CodeDecryptor>,
        redeemer: Arc<dyn CodeRedeemer>,
        dial: Arc<dyn RedeemedDial>,
        self_endpoint: RelayEndpoint,
        observed: String,
    ) {
        // Unseal the realm token with our keypair. A token not sealed to us (or
        // malformed) is not ours to act on.
        let code = match decryptor.decrypt(&observed) {
            Some(code) => code,
            None => {
                tracing::debug!("relay observe: token not sealed to us; ignoring");
                return;
            }
        };
        for (peer_key, world) in peer_manager.pending_offer_peers() {
            let (host, http_port) = match Self::split_endpoint(&peer_key) {
                Some(parts) => parts,
                None => continue,
            };
            match redeemer
                .redeem(&host, http_port, &code, &self_endpoint)
                .await
            {
                Ok(cred) => {
                    if !server_peer_store.authorize_peer(&peer_key, &world) {
                        tracing::warn!(
                            "relay observe: active-identity cap reached; not authorizing {}",
                            peer_key
                        );
                        return;
                    }
                    if peer_manager.begin_initiator_link(&peer_key, Instant::now()) {
                        dial.dial_with_cert(peer_key.clone(), world.clone(), cred);
                    }
                    return;
                }
                Err(e) => {
                    tracing::debug!(
                        "relay observe: redeem at {} rejected ({}); trying next minter",
                        peer_key,
                        e
                    );
                }
            }
        }
    }
}

impl ObservedCodeHandler for ProductionObservedCodeHandler {
    fn on_observed(&self, token: String) {
        let peer_manager = self.peer_manager.clone();
        let server_peer_store = self.server_peer_store.clone();
        let decryptor = self.decryptor.clone();
        let redeemer = self.redeemer.clone();
        let dial = self.dial.clone();
        let self_endpoint = self.self_endpoint.clone();
        tokio::spawn(async move {
            Self::redeem_and_link(
                peer_manager,
                server_peer_store,
                decryptor,
                redeemer,
                dial,
                self_endpoint,
                token,
            )
            .await;
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::relay::peer::link::ingest_sink::RelayIngestSink;
    use crate::relay::peer::table::PeerTable;
    use crate::relay::presence::gate::NeverProven;
    use crate::runtime::ca_cert::CaCertManager;
    use crate::services::CertificateService;
    use common::structs::packet::QuicNetworkPacket;
    use std::fs;
    use std::sync::Mutex as StdMutex;
    use tempfile::TempDir;

    fn ep(host: &str, port: u16) -> RelayEndpoint {
        RelayEndpoint {
            host: host.into(),
            port,
            primary: false,
        }
    }

    struct NoopSink;
    #[async_trait]
    impl RelayIngestSink for NoopSink {
        async fn publish(&self, _packet: QuicNetworkPacket) {}
    }

    // Accepts a redemption only from the one minter host that issued the code;
    // every other minter is rejected (NotFound), exactly as the real store does.
    struct OneMinterRedeemer {
        accepts_host: String,
    }
    #[async_trait]
    impl CodeRedeemer for OneMinterRedeemer {
        async fn redeem(
            &self,
            minter_host: &str,
            _minter_http_port: u16,
            _code: &str,
            _presenter: &RelayEndpoint,
        ) -> Result<PeerCertResponse, anyhow::Error> {
            if minter_host == self.accepts_host {
                Ok(PeerCertResponse {
                    ca_pem: "CA".into(),
                    cert_pem: "CERT".into(),
                    key_pem: "KEY".into(),
                })
            } else {
                Err(anyhow::anyhow!("peer-redeem returned 404 Not Found"))
            }
        }
    }

    #[derive(Default)]
    struct CaptureDial {
        dialed: StdMutex<Vec<(String, String)>>,
    }
    impl RedeemedDial for CaptureDial {
        fn dial_with_cert(&self, peer_ep: String, hashed_world: String, _cred: PeerCertResponse) {
            self.dialed.lock().unwrap().push((peer_ep, hashed_world));
        }
    }

    // Passthrough decryptor: the observed token IS the code (the sealed-box unseal
    // is exercised end-to-end by the e2e; here we test the minter-selection logic).
    struct PassthroughDecryptor;
    impl CodeDecryptor for PassthroughDecryptor {
        fn decrypt(&self, observed: &str) -> Option<String> {
            Some(observed.to_string())
        }
    }

    fn store() -> (Arc<ServerPeerStore>, TempDir) {
        let dir = TempDir::new().expect("tempdir");
        let path = dir.path().to_str().expect("utf-8 path");
        CaCertManager::new(path)
            .ensure(&[String::from("localhost")])
            .expect("CA");
        let ca_pem = fs::read_to_string(format!("{path}/ca.crt")).expect("ca.crt");
        let cert_service = Arc::new(CertificateService::new(path).expect("cert service"));
        (ServerPeerStore::new_shared(cert_service, ca_pem), dir)
    }

    #[tokio::test]
    async fn redeems_against_the_issuing_minter_then_authorizes_and_dials() {
        let table = PeerTable::new_shared();
        let pm =
            PeerManager::new_shared(ep("a", 1), table, Arc::new(NoopSink), Arc::new(NeverProven));
        // Two outstanding offers; only "minter:6000" issued this code.
        let now = Instant::now();
        pm.record_offer("decoy:5000", "W", now);
        pm.record_offer("minter:6000", "W", now);

        let (store, _dir) = store();
        let redeemer = Arc::new(OneMinterRedeemer {
            accepts_host: "minter".into(),
        });
        let dial = Arc::new(CaptureDial::default());

        ProductionObservedCodeHandler::redeem_and_link(
            pm.clone(),
            store.clone(),
            Arc::new(PassthroughDecryptor),
            redeemer,
            dial.clone(),
            ep("a", 1),
            "code123".into(),
        )
        .await;

        // The issuing minter is authorized for the shared world (bidirectional gate).
        assert_eq!(
            store
                .authorized_world("minter:6000", Instant::now())
                .as_deref(),
            Some("W")
        );
        // The decoy minter is never authorized.
        assert_eq!(store.authorized_world("decoy:5000", Instant::now()), None);
        // Exactly the issuing minter is dialed, scoped to W.
        assert_eq!(
            dial.dialed.lock().unwrap().clone(),
            vec![("minter:6000".to_string(), "W".to_string())]
        );
        assert!(pm.has_link("minter:6000"));
    }

    #[tokio::test]
    async fn no_minter_accepts_the_code_then_nothing_is_authorized_or_dialed() {
        let table = PeerTable::new_shared();
        let pm =
            PeerManager::new_shared(ep("a", 1), table, Arc::new(NoopSink), Arc::new(NeverProven));
        pm.record_offer("decoy:5000", "W", Instant::now());

        let (store, _dir) = store();
        let redeemer = Arc::new(OneMinterRedeemer {
            accepts_host: "someone-else".into(),
        });
        let dial = Arc::new(CaptureDial::default());

        ProductionObservedCodeHandler::redeem_and_link(
            pm.clone(),
            store.clone(),
            Arc::new(PassthroughDecryptor),
            redeemer,
            dial.clone(),
            ep("a", 1),
            "forged-or-stale".into(),
        )
        .await;

        assert_eq!(store.authorized_world("decoy:5000", Instant::now()), None);
        assert!(dial.dialed.lock().unwrap().is_empty());
        assert!(!pm.has_link("decoy:5000"));
    }
}
