use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use cached::{Cached, TimedCache, TimedSizedCache};
use tokio::sync::Mutex;

use common::structs::relay::RelayEndpoint;

const WORLD_TTL_SECS: u64 = 3600;

// How long an issued registration challenge token stays valid. The
// registrant must complete the endpoint-control proof and register within this
// window, after which the token expires and a fresh challenge is required.
const CHALLENGE_TTL: Duration = Duration::from_secs(120);

// Upper bound on concurrently outstanding challenges. Caps memory under a flood
// of `issue_challenge` calls; oldest entries are evicted past this size.
const MAX_OUTSTANDING_CHALLENGES: usize = 4096;

#[derive(Clone)]
struct Entry {
    endpoint: RelayEndpoint,
    expires_at: Instant,
}

type WorldEndpoints = HashMap<String, Entry>;

// An outstanding registration challenge: the endpoint it is bound
// to, the nonce the registrant must serve back from that endpoint, whether the
// relay has confirmed the endpoint served the nonce, and when it expires.
#[derive(Clone)]
struct Challenge {
    endpoint_key: String,
    nonce: String,
    verified: bool,
    expires_at: Instant,
}

// HTTPS reachability check. The relay proves the registrant controls the
// endpoint it claims by fetching `/relay/proof/<nonce>` from that endpoint and
// confirming the served body matches the nonce.
#[async_trait::async_trait]
pub trait EndpointReachability: Send + Sync {
    async fn serves_nonce(&self, endpoint: &RelayEndpoint, nonce: &str) -> bool;
}

#[derive(Clone)]
pub struct RelayRegistry {
    store: Arc<Mutex<TimedCache<String, WorldEndpoints>>>,
    // token -> outstanding challenge. Bounds register/lookup to a
    // registrant that proved control of the claimed endpoint. Bounded by
    // capacity and TTL so a flood of challenge requests cannot grow it without
    // limit.
    challenges: Arc<Mutex<TimedSizedCache<String, Challenge>>>,
}

impl RelayRegistry {
    pub fn new() -> Self {
        let store = TimedCache::with_lifespan_and_refresh(Duration::from_secs(WORLD_TTL_SECS), true);
        let challenges =
            TimedSizedCache::with_size_and_lifespan(MAX_OUTSTANDING_CHALLENGES, CHALLENGE_TTL);
        Self {
            store: Arc::new(Mutex::new(store)),
            challenges: Arc::new(Mutex::new(challenges)),
        }
    }

    pub fn new_shared() -> Arc<Self> {
        Arc::new(Self::new())
    }

    fn endpoint_key(ep: &RelayEndpoint) -> String {
        format!("{}:{}", ep.host, ep.port)
    }

    // Issues an endpoint-bound registration challenge. Returns the
    // `(token, nonce)` the registrant uses: it must serve `nonce` at the claimed
    // endpoint so `verify_endpoint` can confirm control, then present `token` to
    // `register`. The challenge starts unverified.
    pub async fn issue_challenge(&self, endpoint: &RelayEndpoint, now: Instant) -> (String, String) {
        let token = nanoid::nanoid!(32);
        let nonce = nanoid::nanoid!(32);
        let mut challenges = self.challenges.lock().await;
        challenges.cache_set(
            token.clone(),
            Challenge {
                endpoint_key: Self::endpoint_key(endpoint),
                nonce: nonce.clone(),
                verified: false,
                expires_at: now + CHALLENGE_TTL,
            },
        );
        (token, nonce)
    }

    // Completes the endpoint-control proof for `token` by asking `reachability`
    // to confirm the bound endpoint serves the challenge nonce.
    // Marks the challenge verified on success. Returns whether verification
    // succeeded. The actual HTTPS fetch lives behind the `EndpointReachability`
    // seam (the live-socket boundary).
    pub async fn verify_endpoint(
        &self,
        token: &str,
        endpoint: &RelayEndpoint,
        reachability: &dyn EndpointReachability,
        now: Instant,
    ) -> bool {
        let nonce = {
            let mut challenges = self.challenges.lock().await;
            match challenges.cache_get(token) {
                Some(c)
                    if c.expires_at > now
                        && c.endpoint_key == Self::endpoint_key(endpoint) =>
                {
                    c.nonce.clone()
                }
                _ => return false,
            }
        };

        if !reachability.serves_nonce(endpoint, &nonce).await {
            return false;
        }

        let mut challenges = self.challenges.lock().await;
        if let Some(c) = challenges.cache_get_mut(token) {
            if c.expires_at > now && c.endpoint_key == Self::endpoint_key(endpoint) {
                c.verified = true;
                return true;
            }
        }
        false
    }

    // True when `token` is a live, verified challenge bound to `endpoint`.
    // Register/lookup are gated on this so an attacker who merely
    // knows `H(relay_world_uuid)` cannot inject a victim's (or a fake) endpoint
    // — they would have to control the endpoint to pass `verify_endpoint`.
    async fn token_authorizes(&self, token: &str, endpoint: &RelayEndpoint, now: Instant) -> bool {
        if token.is_empty() {
            return false;
        }
        let mut challenges = self.challenges.lock().await;
        match challenges.cache_get(token) {
            Some(c) => {
                c.verified
                    && c.expires_at > now
                    && c.endpoint_key == Self::endpoint_key(endpoint)
            }
            None => false,
        }
    }

    // Registers `endpoint` into `hashed_world`, gated on a verified,
    // endpoint-bound `token`. Returns false (and registers nothing)
    // when the token does not authorize this endpoint — default deny.
    pub async fn register(
        &self,
        hashed_world: &str,
        ep: RelayEndpoint,
        ttl_secs: u32,
        token: &str,
        now: Instant,
    ) -> bool {
        if !self.token_authorizes(token, &ep, now).await {
            tracing::warn!(
                "relay register denied for {} (unproven endpoint token)",
                Self::endpoint_key(&ep)
            );
            return false;
        }

        let entry = Entry {
            expires_at: now + Duration::from_secs(ttl_secs as u64),
            endpoint: ep.clone(),
        };

        let mut store = self.store.lock().await;
        let mut world = store.cache_get(hashed_world).cloned().unwrap_or_default();
        world.insert(Self::endpoint_key(&ep), entry);
        store.cache_set(hashed_world.to_string(), world);
        true
    }

    // Returns the peers (self-excluded) in each requested world the caller is
    // registered in. Gated on the same endpoint-control-proven `token` register
    // requires: the caller must control the `caller`
    // endpoint it claims, so an attacker who merely knows a world hash AND a
    // registered member endpoint cannot pass `caller = victim_endpoint` to
    // enumerate. An unauthorized token yields an empty result (default deny).
    pub async fn lookup(
        &self,
        caller: &RelayEndpoint,
        hashed_worlds: &[String],
        token: &str,
        now: Instant,
    ) -> HashMap<String, Vec<RelayEndpoint>> {
        let mut result = HashMap::new();

        if !self.token_authorizes(token, caller, now).await {
            tracing::warn!(
                "relay lookup denied for {} (unproven endpoint token)",
                Self::endpoint_key(caller)
            );
            return result;
        }

        let caller_key = Self::endpoint_key(caller);

        let mut store = self.store.lock().await;
        for hashed_world in hashed_worlds {
            let Some(world) = store.cache_get(hashed_world).cloned() else {
                continue;
            };

            let live: WorldEndpoints = world
                .into_iter()
                .filter(|(_, entry)| entry.expires_at > now)
                .collect();

            if !live.contains_key(&caller_key) {
                store.cache_set(hashed_world.to_string(), live);
                continue;
            }

            let peers: Vec<RelayEndpoint> = live
                .iter()
                .filter(|(key, _)| key.as_str() != caller_key)
                .map(|(_, entry)| entry.endpoint.clone())
                .collect();

            store.cache_set(hashed_world.to_string(), live);
            result.insert(hashed_world.clone(), peers);
        }

        result
    }
}

impl Default for RelayRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ep(host: &str, port: u16) -> RelayEndpoint {
        RelayEndpoint {
            host: host.to_string(),
            port,
            primary: false,
        }
    }

    // Reachability stub: confirms the nonce iff the endpoint is in its allow-set
    // (models "this attacker does/doesn't actually control the endpoint").
    struct StubReachability {
        controlled: Vec<String>,
    }

    #[async_trait::async_trait]
    impl EndpointReachability for StubReachability {
        async fn serves_nonce(&self, endpoint: &RelayEndpoint, _nonce: &str) -> bool {
            self.controlled
                .contains(&RelayRegistry::endpoint_key(endpoint))
        }
    }

    // Issues a challenge, proves control via the stub, and returns the token.
    async fn proven_token(
        reg: &RelayRegistry,
        endpoint: &RelayEndpoint,
        reach: &dyn EndpointReachability,
        now: Instant,
    ) -> String {
        let (token, _nonce) = reg.issue_challenge(endpoint, now).await;
        assert!(reg.verify_endpoint(&token, endpoint, reach, now).await);
        token
    }

    #[tokio::test]
    async fn register_additive_lookup_scoped_to_caller_and_excludes_self() {
        let reg = RelayRegistry::new();
        let a = ep("a", 1);
        let b = ep("b", 2);
        let reach = StubReachability {
            controlled: vec!["a:1".into(), "b:2".into(), "c:3".into()],
        };
        let now = Instant::now();
        let ta = proven_token(&reg, &a, &reach, now).await;
        let tb = proven_token(&reg, &b, &reach, now).await;
        assert!(reg.register("hW", a.clone(), 60, &ta, now).await);
        assert!(reg.register("hW", b.clone(), 60, &tb, now).await);

        let got = reg.lookup(&a, &["hW".to_string()], &ta, now).await;
        assert_eq!(got.get("hW").unwrap(), &vec![b.clone()]);

        let c = ep("c", 3);
        let tc = proven_token(&reg, &c, &reach, now).await;
        assert!(reg
            .lookup(&c, &["hW".to_string()], &tc, now)
            .await
            .get("hW")
            .map(|x| x.is_empty())
            .unwrap_or(true));
    }

    #[tokio::test]
    async fn register_is_additive_per_endpoint_upsert() {
        let reg = RelayRegistry::new();
        let a = ep("a", 1);
        let b = ep("b", 2);
        let reach = StubReachability {
            controlled: vec!["a:1".into(), "b:2".into()],
        };
        let now = Instant::now();
        let ta = proven_token(&reg, &a, &reach, now).await;
        let tb = proven_token(&reg, &b, &reach, now).await;
        assert!(reg.register("hW", a.clone(), 60, &ta, now).await);
        assert!(reg.register("hW", b.clone(), 60, &tb, now).await);
        let ta2 = proven_token(&reg, &a, &reach, now).await;
        assert!(reg.register("hW", a.clone(), 60, &ta2, now).await);

        let got = reg.lookup(&b, &["hW".to_string()], &tb, now).await;
        assert_eq!(got.get("hW").unwrap(), &vec![a.clone()]);
    }

    #[tokio::test]
    async fn lookup_of_unknown_world_returns_empty() {
        let reg = RelayRegistry::new();
        let a = ep("a", 1);
        let reach = StubReachability {
            controlled: vec!["a:1".into()],
        };
        let now = Instant::now();
        let ta = proven_token(&reg, &a, &reach, now).await;
        let got = reg
            .lookup(&a, &["never-registered".to_string()], &ta, now)
            .await;
        assert!(got.is_empty());
    }

    #[tokio::test]
    async fn expired_endpoints_are_excluded() {
        let reg = RelayRegistry::new();
        let a = ep("a", 1);
        let b = ep("b", 2);
        let reach = StubReachability {
            controlled: vec!["a:1".into(), "b:2".into()],
        };
        let now = Instant::now();
        let ta = proven_token(&reg, &a, &reach, now).await;
        let tb = proven_token(&reg, &b, &reach, now).await;
        assert!(reg.register("hW", a.clone(), 60, &ta, now).await);
        assert!(reg.register("hW", b.clone(), 0, &tb, now).await);

        let got = reg.lookup(&a, &["hW".to_string()], &ta, now).await;
        assert!(got.get("hW").unwrap().is_empty());
    }

    // An attacker who knows the world hash but does NOT control the
    // endpoint it claims cannot register — the endpoint-control proof fails, no
    // token is issued, and register is denied. It therefore cannot then enumerate
    // peers via lookup (it is never in the world).
    #[tokio::test]
    async fn unproven_endpoint_register_is_rejected() {
        let reg = RelayRegistry::new();
        let victim = ep("victim", 9000);
        let attacker = ep("attacker", 6666);
        // The attacker controls only its own endpoint, not the victim's.
        let reach = StubReachability {
            controlled: vec!["attacker:6666".into()],
        };
        let now = Instant::now();

        // Attacker tries to register the VICTIM's endpoint (to point a world at
        // it / squat it): challenge issues, but verification fails (attacker does
        // not control victim), so no usable token exists and register is denied.
        let (vtoken, _) = reg.issue_challenge(&victim, now).await;
        assert!(
            !reg.verify_endpoint(&vtoken, &victim, &reach, now).await,
            "attacker must not be able to prove control of the victim's endpoint"
        );
        assert!(
            !reg.register("hW", victim.clone(), 60, &vtoken, now).await,
            "register of an unproven endpoint must be denied"
        );

        // And a register with no/garbage token is denied outright.
        assert!(!reg.register("hW", attacker.clone(), 60, "", now).await);
        assert!(!reg.register("hW", attacker.clone(), 60, "garbage", now).await);
    }

    // A token issued for one endpoint cannot be replayed to register
    // a DIFFERENT endpoint (token is bound to the endpoint it was issued for).
    #[tokio::test]
    async fn token_is_bound_to_its_endpoint() {
        let reg = RelayRegistry::new();
        let mine = ep("attacker", 6666);
        let victim = ep("victim", 9000);
        let reach = StubReachability {
            controlled: vec!["attacker:6666".into()],
        };
        let now = Instant::now();
        // Attacker legitimately proves its OWN endpoint.
        let token = proven_token(&reg, &mine, &reach, now).await;
        // ...then tries to use that token to register the victim's endpoint.
        assert!(
            !reg.register("hW", victim, 60, &token, now).await,
            "a token proven for one endpoint must not authorize another"
        );
        // The token still works for the endpoint it was issued for.
        assert!(reg.register("hW", mine, 60, &token, now).await);
    }

    // Lookup is gated on the same endpoint-control proof
    // register requires. A caller with a missing, garbage, or mismatched token
    // gets nothing back (default deny); a caller presenting a token proven for
    // the endpoint it claims gets the scoped, self-excluded peer set.
    #[tokio::test]
    async fn lookup_requires_endpoint_control_token() {
        let reg = RelayRegistry::new();
        let a = ep("a", 1);
        let b = ep("b", 2);
        let attacker = ep("attacker", 6666);
        let reach = StubReachability {
            controlled: vec!["a:1".into(), "b:2".into(), "attacker:6666".into()],
        };
        let now = Instant::now();
        let ta = proven_token(&reg, &a, &reach, now).await;
        let tb = proven_token(&reg, &b, &reach, now).await;
        assert!(reg.register("hW", a.clone(), 60, &ta, now).await);
        assert!(reg.register("hW", b.clone(), 60, &tb, now).await);

        // Missing token -> denied (empty result).
        assert!(reg.lookup(&a, &["hW".to_string()], "", now).await.is_empty());
        // Garbage token -> denied.
        assert!(reg
            .lookup(&a, &["hW".to_string()], "garbage", now)
            .await
            .is_empty());

        // Endpoint-mismatched token: the attacker proves its OWN endpoint, then
        // tries to enumerate by claiming `caller = a` (a registered member). The
        // token is not bound to `a`, so lookup is denied.
        let t_attacker = proven_token(&reg, &attacker, &reach, now).await;
        assert!(
            reg.lookup(&a, &["hW".to_string()], &t_attacker, now)
                .await
                .is_empty(),
            "a token proven for one endpoint must not authorize lookup as another"
        );

        // Valid token for the claimed endpoint -> scoped, self-excluded peers.
        let got = reg.lookup(&a, &["hW".to_string()], &ta, now).await;
        assert_eq!(got.get("hW").unwrap(), &vec![b.clone()]);
    }

    // An expired challenge token no longer authorizes register.
    #[tokio::test]
    async fn expired_challenge_token_is_rejected() {
        let reg = RelayRegistry::new();
        let a = ep("a", 1);
        let reach = StubReachability {
            controlled: vec!["a:1".into()],
        };
        let t0 = Instant::now();
        let token = proven_token(&reg, &a, &reach, t0).await;
        let later = t0 + CHALLENGE_TTL + Duration::from_secs(1);
        assert!(
            !reg.register("hW", a, 60, &token, later).await,
            "an expired challenge token must not authorize register"
        );
    }
}
