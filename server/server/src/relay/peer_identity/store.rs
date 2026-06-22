use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::services::CertificateService;

use super::error::RedeemError;
use super::identity::RedeemedPeerIdentity;

// One outstanding code: who may redeem it, the credential it yields, when it
// expires, and whether it has been consumed.
struct PendingCode {
    recipient: String,
    identity: RedeemedPeerIdentity,
    expires_at: Instant,
    consumed: bool,
}

// Lifecycle of a redeemed peer identity. `Connected` is authorized; `Grace`
// keeps authorization briefly after a link drop so a transient reconnect does
// not require a fresh code; past the grace deadline the identity is invalid and
// is swept (its cert/keys forgotten — re-establishment needs a new code).
enum LinkState {
    Connected,
    Grace { until: Instant },
}

// A redeemed, in-memory peer identity and the world it is authorized to relay.
struct ActiveIdentity {
    world: String,
    state: LinkState,
}

// In-memory store of server-peer codes and the ephemeral identities they mint.
//
// Distinct from the DB-backed player `AuthCodeService`: nothing here touches the
// database, so a reboot clears every code and identity (forcing peers to
// re-establish). Codes are single-use (atomic check-and-consume) and
// recipient-bound (only the endpoint a code was minted for may redeem it).
// DoS bounds: a flood of offers (each minting a code) or redemptions
// (each minting an in-memory identity) must not exhaust memory. mint/redeem
// fail-closed once at capacity rather than growing without bound. The defaults
// are far above any legitimate cross-server topology (one code/identity per
// peer per world).
const DEFAULT_MAX_PENDING_CODES: usize = 1024;
const DEFAULT_MAX_ACTIVE_PEERS: usize = 512;

pub struct ServerPeerStore {
    cert_service: Arc<CertificateService>,
    ca_pem: String,
    max_pending: usize,
    max_active: usize,
    pending: Mutex<HashMap<String, PendingCode>>,
    // Redeemed identities keyed by peer endpoint (`host:port`). The authorization
    // gate consults this to decide whether a peer may relay a world's audio.
    active: Mutex<HashMap<String, ActiveIdentity>>,
}

impl ServerPeerStore {
    // How long a redeemed identity stays authorized after its link drops, so a
    // transient blip reconnects without a fresh code. Past this it is swept and
    // re-establishment needs a new Flow-1 offer.
    pub const RECONNECT_GRACE: Duration = Duration::from_secs(30);

    pub fn new(cert_service: Arc<CertificateService>, ca_pem: String) -> Self {
        Self::with_caps(
            cert_service,
            ca_pem,
            DEFAULT_MAX_PENDING_CODES,
            DEFAULT_MAX_ACTIVE_PEERS,
        )
    }

    fn with_caps(
        cert_service: Arc<CertificateService>,
        ca_pem: String,
        max_pending: usize,
        max_active: usize,
    ) -> Self {
        Self {
            cert_service,
            ca_pem,
            max_pending,
            max_active,
            pending: Mutex::new(HashMap::new()),
            active: Mutex::new(HashMap::new()),
        }
    }

    pub fn new_shared(cert_service: Arc<CertificateService>, ca_pem: String) -> Arc<Self> {
        Arc::new(Self::new(cert_service, ca_pem))
    }

    // Mints a single-use code bound to the recipient endpoint `{host}:{port}`,
    // issuing a `server::{host}:{port}`-CN peer cert authorized for `world`. Only
    // that endpoint may later redeem the code (recipient-binding).
    pub fn mint(
        &self,
        world: &str,
        recipient_host: &str,
        recipient_port: u16,
        ttl: Duration,
        now: Instant,
    ) -> Result<String, anyhow::Error> {
        // DoS bound: refuse before the expensive cert signing when the
        // outstanding-code table is already at capacity.
        {
            let pending = self.pending.lock().expect("peer code store poisoned");
            if pending.len() >= self.max_pending {
                anyhow::bail!("peer code store at capacity ({} pending)", self.max_pending);
            }
        }
        let (cert, key) = self
            .cert_service
            .sign_peer_cert(recipient_host, recipient_port)?;
        let endpoint = format!("{recipient_host}:{recipient_port}");
        let identity = RedeemedPeerIdentity {
            endpoint: endpoint.clone(),
            world: world.to_string(),
            ca_pem: self.ca_pem.clone(),
            cert_pem: cert.pem(),
            key_pem: key.serialize_pem(),
        };

        let code = nanoid::nanoid!(32);
        self.pending
            .lock()
            .expect("peer code store poisoned")
            .insert(
                code.clone(),
                PendingCode {
                    recipient: endpoint,
                    identity,
                    expires_at: now + ttl,
                    consumed: false,
                },
            );
        Ok(code)
    }

    // Redeems a code exactly once. The `presenter` (the redeeming endpoint,
    // authenticated by the redemption channel) must match the bound recipient.
    // Atomic check-and-consume: a second redemption — or a concurrent one — sees
    // `consumed` and is refused. A wrong-recipient or expired attempt does NOT
    // consume the code, so the legitimate recipient can still redeem.
    pub fn redeem(
        &self,
        code: &str,
        presenter: &str,
        now: Instant,
    ) -> Result<RedeemedPeerIdentity, RedeemError> {
        let mut pending = self.pending.lock().expect("peer code store poisoned");
        let entry = pending.get_mut(code).ok_or(RedeemError::NotFound)?;
        if now >= entry.expires_at {
            return Err(RedeemError::Expired);
        }
        if entry.recipient != presenter {
            return Err(RedeemError::WrongRecipient);
        }
        if entry.consumed {
            return Err(RedeemError::AlreadyUsed);
        }
        // DoS bound: refuse a NEW active identity once at capacity (a
        // reconnecting, already-active endpoint is exempt). Checked before
        // consuming the code so the legitimate recipient can retry once capacity
        // frees.
        {
            let active = self.active.lock().expect("peer code store poisoned");
            if active.len() >= self.max_active && !active.contains_key(presenter) {
                return Err(RedeemError::AtCapacity);
            }
        }
        entry.consumed = true;
        let identity = entry.identity.clone();
        drop(pending);

        // The redeemed peer is now authorized to relay its world until its link
        // drops and the reconnect grace lapses.
        self.active
            .lock()
            .expect("peer code store poisoned")
            .insert(
                identity.endpoint.clone(),
                ActiveIdentity {
                    world: identity.world.clone(),
                    state: LinkState::Connected,
                },
            );
        Ok(identity)
    }

    // The world a peer endpoint is currently authorized to relay (in and out), or
    // `None` if it is unknown, or its reconnect grace has lapsed. This is the
    // authorization the relay gate consults for every peer packet.
    pub fn authorized_world(&self, endpoint: &str, now: Instant) -> Option<String> {
        let active = self.active.lock().expect("peer code store poisoned");
        let id = active.get(endpoint)?;
        match id.state {
            LinkState::Connected => Some(id.world.clone()),
            LinkState::Grace { until } if now < until => Some(id.world.clone()),
            LinkState::Grace { .. } => None,
        }
    }

    // The peer link dropped: keep authorization for a brief reconnect grace so a
    // transient blip does not force a fresh code. After the grace deadline the
    // identity is invalid (and `sweep` forgets it).
    pub fn mark_disconnected(&self, endpoint: &str, now: Instant, grace: Duration) {
        if let Some(id) = self
            .active
            .lock()
            .expect("peer code store poisoned")
            .get_mut(endpoint)
        {
            id.state = LinkState::Grace { until: now + grace };
        }
    }

    // Authorizes a peer endpoint for a world WITHOUT minting a code. The asker
    // uses this after it redeems a minter's code and dials: both sides proved
    // realm presence (the minter could inject the code; we observed it), so we
    // authorize the minter to relay back to us. Idempotent; starts `Connected`.
    // Returns false (no-op) when at the active-identity cap and this is a NEW
    // endpoint, so the asker side honors the same DoS bound as `redeem`.
    pub fn authorize_peer(&self, endpoint: &str, world: &str) -> bool {
        let mut active = self.active.lock().expect("peer code store poisoned");
        if active.len() >= self.max_active && !active.contains_key(endpoint) {
            return false;
        }
        active.insert(
            endpoint.to_string(),
            ActiveIdentity {
                world: world.to_string(),
                state: LinkState::Connected,
            },
        );
        true
    }

    // The peer reconnected within grace: cancel the grace timer, fully authorized.
    pub fn mark_reconnected(&self, endpoint: &str) {
        if let Some(id) = self
            .active
            .lock()
            .expect("peer code store poisoned")
            .get_mut(endpoint)
        {
            id.state = LinkState::Connected;
        }
    }

    // Forgets identities whose reconnect grace has lapsed (cert/keys dropped).
    pub fn sweep(&self, now: Instant) {
        self.active
            .lock()
            .expect("peer code store poisoned")
            .retain(|_, id| match id.state {
                LinkState::Connected => true,
                LinkState::Grace { until } => now < until,
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::ca_cert::CaCertManager;
    use std::fs;
    use tempfile::TempDir;

    fn store() -> (ServerPeerStore, TempDir) {
        let dir = TempDir::new().expect("tempdir");
        let path = dir.path().to_str().expect("utf-8 path");
        CaCertManager::new(path)
            .ensure(&[String::from("localhost")])
            .expect("CA generation");
        let ca_pem = fs::read_to_string(format!("{path}/ca.crt")).expect("read ca.crt");
        let cert_service =
            Arc::new(CertificateService::new(path).expect("CertificateService::new"));
        (ServerPeerStore::new(cert_service, ca_pem), dir)
    }

    #[test]
    fn mint_then_redeem_once_yields_world_bound_identity() {
        let (store, _dir) = store();
        let t0 = Instant::now();
        let code = store
            .mint("W", "asker.host", 6000, Duration::from_secs(180), t0)
            .expect("mint");

        let id = store
            .redeem(&code, "asker.host:6000", t0)
            .expect("legit recipient redeems");
        assert_eq!(id.endpoint, "asker.host:6000");
        assert_eq!(id.world, "W");
        assert!(id.cert_pem.contains("BEGIN CERTIFICATE"));
        assert!(id.key_pem.contains("PRIVATE KEY"));
        assert!(id.ca_pem.contains("BEGIN CERTIFICATE"));
    }

    #[test]
    fn code_is_single_use() {
        let (store, _dir) = store();
        let t0 = Instant::now();
        let code = store
            .mint("W", "asker.host", 6000, Duration::from_secs(180), t0)
            .expect("mint");
        assert!(store.redeem(&code, "asker.host:6000", t0).is_ok());
        assert!(
            matches!(
                store.redeem(&code, "asker.host:6000", t0),
                Err(RedeemError::AlreadyUsed)
            ),
            "a server-peer code must not redeem twice"
        );
    }

    #[test]
    fn wrong_recipient_is_refused_and_does_not_consume() {
        let (store, _dir) = store();
        let t0 = Instant::now();
        let code = store
            .mint("W", "asker.host", 6000, Duration::from_secs(180), t0)
            .expect("mint");
        // A forwarded code presented by anyone but the bound recipient is refused.
        assert!(matches!(
            store.redeem(&code, "attacker.host:9999", t0),
            Err(RedeemError::WrongRecipient)
        ));
        // ...and the legitimate recipient can still redeem (not consumed).
        assert!(store.redeem(&code, "asker.host:6000", t0).is_ok());
    }

    #[test]
    fn expired_code_is_refused() {
        let (store, _dir) = store();
        let t0 = Instant::now();
        let code = store
            .mint("W", "asker.host", 6000, Duration::from_secs(180), t0)
            .expect("mint");
        let later = t0 + Duration::from_secs(181);
        assert!(matches!(
            store.redeem(&code, "asker.host:6000", later),
            Err(RedeemError::Expired)
        ));
    }

    #[test]
    fn unknown_code_is_not_found() {
        let (store, _dir) = store();
        assert!(matches!(
            store.redeem("nope", "asker.host:6000", Instant::now()),
            Err(RedeemError::NotFound)
        ));
    }

    // Helper: mint + redeem so an endpoint is active, returning the store.
    fn active_store() -> (ServerPeerStore, TempDir, String, Instant) {
        let (store, dir) = store();
        let t0 = Instant::now();
        let code = store
            .mint("W", "asker.host", 6000, Duration::from_secs(180), t0)
            .expect("mint");
        store.redeem(&code, "asker.host:6000", t0).expect("redeem");
        (store, dir, "asker.host:6000".to_string(), t0)
    }

    #[test]
    fn redeem_authorizes_endpoint_for_its_world() {
        let (store, _dir, ep, t0) = active_store();
        assert_eq!(store.authorized_world(&ep, t0).as_deref(), Some("W"));
    }

    #[test]
    fn within_grace_stays_authorized_then_lapses() {
        let (store, _dir, ep, t0) = active_store();
        store.mark_disconnected(&ep, t0, Duration::from_secs(5));
        assert_eq!(
            store
                .authorized_world(&ep, t0 + Duration::from_secs(4))
                .as_deref(),
            Some("W"),
            "within the reconnect grace the peer stays authorized"
        );
        assert_eq!(
            store.authorized_world(&ep, t0 + Duration::from_secs(6)),
            None,
            "past the grace deadline the peer is no longer authorized"
        );
    }

    #[test]
    fn reconnect_within_grace_revalidates() {
        let (store, _dir, ep, t0) = active_store();
        store.mark_disconnected(&ep, t0, Duration::from_secs(5));
        store.mark_reconnected(&ep);
        assert_eq!(
            store
                .authorized_world(&ep, t0 + Duration::from_secs(60))
                .as_deref(),
            Some("W"),
            "a reconnect within grace cancels the grace timer"
        );
    }

    #[test]
    fn sweep_forgets_grace_lapsed_identities() {
        let (store, _dir, ep, t0) = active_store();
        store.mark_disconnected(&ep, t0, Duration::from_secs(5));
        store.sweep(t0 + Duration::from_secs(6));
        assert_eq!(
            store.authorized_world(&ep, t0 + Duration::from_secs(6)),
            None,
            "a swept identity's cert/keys are forgotten"
        );
    }

    #[test]
    fn unknown_endpoint_is_not_authorized() {
        let (store, _dir) = store();
        assert_eq!(store.authorized_world("nobody:1", Instant::now()), None);
    }

    fn store_with_caps(max_pending: usize, max_active: usize) -> (ServerPeerStore, TempDir) {
        let dir = TempDir::new().expect("tempdir");
        let path = dir.path().to_str().expect("utf-8 path");
        CaCertManager::new(path)
            .ensure(&[String::from("localhost")])
            .expect("CA generation");
        let ca_pem = fs::read_to_string(format!("{path}/ca.crt")).expect("read ca.crt");
        let cert_service =
            Arc::new(CertificateService::new(path).expect("CertificateService::new"));
        (
            ServerPeerStore::with_caps(cert_service, ca_pem, max_pending, max_active),
            dir,
        )
    }

    #[test]
    fn mint_refuses_once_pending_codes_hit_the_cap() {
        let (store, _dir) = store_with_caps(2, 16);
        let t0 = Instant::now();
        let ttl = Duration::from_secs(180);
        assert!(store.mint("W", "h1", 6000, ttl, t0).is_ok());
        assert!(store.mint("W", "h2", 6001, ttl, t0).is_ok());
        // Third offer exceeds the cap and is refused (offer-flood DoS bound).
        assert!(
            store.mint("W", "h3", 6002, ttl, t0).is_err(),
            "mint must fail-closed once the pending-code table is full"
        );
    }

    #[test]
    fn redeem_refuses_a_new_identity_once_active_hits_the_cap() {
        let (store, _dir) = store_with_caps(16, 1);
        let t0 = Instant::now();
        let ttl = Duration::from_secs(180);
        let c1 = store.mint("W", "a", 6000, ttl, t0).expect("mint 1");
        let c2 = store.mint("W", "b", 6001, ttl, t0).expect("mint 2");
        assert!(
            store.redeem(&c1, "a:6000", t0).is_ok(),
            "first identity fits"
        );
        // A second DISTINCT active identity exceeds the cap and is refused without
        // consuming the code (the legit recipient can retry once capacity frees).
        assert!(matches!(
            store.redeem(&c2, "b:6001", t0),
            Err(RedeemError::AtCapacity)
        ));
        // Free capacity by dropping the first identity; the capacity-refused code
        // was NOT consumed, so it still redeems once room frees.
        store.mark_disconnected("a:6000", t0, Duration::from_secs(0));
        store.sweep(t0 + Duration::from_secs(1));
        assert!(
            store
                .redeem(&c2, "b:6001", t0 + Duration::from_secs(1))
                .is_ok(),
            "the capacity-refused code must not have been consumed"
        );
    }

    #[test]
    fn authorize_peer_grants_world_without_a_code() {
        let (store, _dir) = store();
        assert!(store.authorize_peer("minter.host:7000", "W"));
        assert_eq!(
            store
                .authorized_world("minter.host:7000", Instant::now())
                .as_deref(),
            Some("W"),
            "the asker authorizes the minter for the shared world after redeem+dial"
        );
    }
}
