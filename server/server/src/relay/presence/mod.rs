use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use common::structs::relay::RelayEndpoint;

pub mod gate;

pub use gate::{AlwaysProven, NeverProven, PresenceGate};

use crate::relay::peer::manager::PeerManager;

// How long an injected challenge token stays outstanding before it can no
// longer be matched against a peer's echo
pub const CHALLENGE_TTL: Duration = Duration::from_secs(30);

// One outstanding/expected token: the world it proves presence in and when it
// expires.
struct WorldToken {
    hashed_world: String,
    expires_at: Instant,
}

// Drives the in-realm presence proof that gates server↔server
// peering. The security root is structural, not procedural:
//
//   * A challenge token is created by `new_challenge` and is meant to travel
//     ONLY through the realm — the server sends it to its OWN local client(s)
//     as a `PeerPresenceInject`, the client injects it as suppressed chat, and
//     the peer's client observes it. This type therefore exposes NO method that
//     emits a challenge toward a peer link.
//   * The only peer-link interactions are: (a) we RECEIVE a peer's echo of a
//     token we injected (`record_observed_from_peer`), proving the peer has a
//     live client in the world; and (b) we ECHO back a token OUR OWN client
//     observed (`on_client_observed` + `tokens_to_echo_to_peer`), so the peer
//     can complete its own (symmetric) proof of us.
//
// A peer that is not actually present in the world can never echo a token it
// never received, so it can never become proven.
//
// Token validity is enforced on BOTH sides: a token our
// client reports as observed is echoed/recorded ONLY when it matches a token we
// are participating in — either a challenge WE injected (`outstanding`) or one a
// peer is expected to challenge us with (`expected`, registered out-of-band via
// `expect_observed`). An arbitrary attacker-injected `!bvcp` string matches
// neither and is dropped, so it can neither be echoed onward nor advance proof
// state.
pub struct PresenceProver {
    // token -> the challenge we injected (verifier side).
    outstanding: Mutex<HashMap<String, WorldToken>>,
    // token -> a token a peer is expected to challenge us with, that our client
    // will observe via the realm fan-out (prover side). Lets `on_client_observed`
    // accept a legitimate peer-issued token without echoing arbitrary strings.
    expected: Mutex<HashMap<String, WorldToken>>,
    // (peer endpoint key, hashed world) the peer has proven presence in by
    // echoing a token WE injected.
    proven: Mutex<HashSet<(String, String)>>,
    // (token, hashed world) pairs our own client observed in the realm (the peer
    // challenged US); drained by `tokens_to_echo_to_peer` to be echoed back over
    // the peer link, world-attributed so the echo records the correct world.
    to_echo: Mutex<Vec<(String, String)>>,
    // (peer endpoint key, hashed world) we have echoed an observed token back to
    // over the peer link — i.e. WE have completed the peer's proof of US for that
    // world. Mutual proof (`is_mutually_proven`) requires both this AND the peer
    // echoing our token, BOTH scoped to the same world.
    echoed_to_peers: Mutex<HashSet<(String, String)>>,
}

impl PresenceProver {
    pub fn new() -> Self {
        Self {
            outstanding: Mutex::new(HashMap::new()),
            expected: Mutex::new(HashMap::new()),
            proven: Mutex::new(HashSet::new()),
            to_echo: Mutex::new(Vec::new()),
            echoed_to_peers: Mutex::new(HashSet::new()),
        }
    }

    pub fn new_shared() -> std::sync::Arc<Self> {
        std::sync::Arc::new(Self::new())
    }

    // Creates a fresh high-entropy challenge for `hashed_world` and records it as
    // outstanding (TTL from `now`). The returned token is delivered to the
    // server's OWN local client(s) via `PeerPresenceInject` — NEVER to a peer.
    pub fn new_challenge(&self, hashed_world: &str, now: Instant) -> String {
        let token = nanoid::nanoid!(32);
        let mut outstanding = self.outstanding.lock().expect("outstanding poisoned");
        Self::prune_expired(&mut outstanding, now);
        outstanding.insert(
            token.clone(),
            WorldToken {
                hashed_world: hashed_world.to_string(),
                expires_at: now + CHALLENGE_TTL,
            },
        );
        token
    }

    // Registers a token a peer is expected to challenge us with for
    // `hashed_world` (the peer injects it into the realm; the realm fans it out
    // to our client, which reports it via `on_client_observed`). Only an expected
    // (or our own outstanding) token is ever echoed onward — this is what keeps
    // arbitrary strings off the echo path on the observing side.
    pub fn expect_observed(&self, token: &str, hashed_world: &str, now: Instant) {
        let mut expected = self.expected.lock().expect("expected poisoned");
        Self::prune_expired(&mut expected, now);
        expected.insert(
            token.to_string(),
            WorldToken {
                hashed_world: hashed_world.to_string(),
                expires_at: now + CHALLENGE_TTL,
            },
        );
    }

    // A peer echoed `token` back to us over the peer link. If it matches an
    // unexpired challenge WE injected, the peer is marked proven-present for that
    // challenge's world. An unknown or expired token is a no-op.
    pub fn record_observed_from_peer(&self, peer: &str, token: &str, now: Instant) {
        let world = {
            let mut outstanding = self.outstanding.lock().expect("outstanding poisoned");
            Self::prune_expired(&mut outstanding, now);
            match outstanding.get(token) {
                Some(c) => c.hashed_world.clone(),
                None => return,
            }
        };
        self.proven
            .lock()
            .expect("proven poisoned")
            .insert((peer.to_string(), world));
    }

    // Our OWN local client observed `token` in the realm — meaning a peer
    // challenged us. The token is echoed back to the peer over the link ONLY IF
    // it matches a token we are participating in: a challenge WE injected
    // (`outstanding`) or one a peer is `expected` to challenge us with. An
    // arbitrary attacker-injected string matches neither and is silently dropped
    // — it is never queued for echo and never advances proof state.
    // The matched world is carried alongside the token so the echo is
    // world-attributed.
    pub fn on_client_observed(&self, token: &str, now: Instant) {
        let world = match self.known_token_world(token, now) {
            Some(world) => world,
            None => {
                tracing::debug!("dropping observed token with no matching outstanding challenge");
                return;
            }
        };
        self.to_echo
            .lock()
            .expect("to_echo poisoned")
            .push((token.to_string(), world));
    }

    // Looks up the world a token belongs to across our outstanding challenges and
    // the expected-peer-token set, pruning expired entries. `None` when the token
    // matches no known, unexpired challenge.
    fn known_token_world(&self, token: &str, now: Instant) -> Option<String> {
        {
            let mut outstanding = self.outstanding.lock().expect("outstanding poisoned");
            Self::prune_expired(&mut outstanding, now);
            if let Some(c) = outstanding.get(token) {
                return Some(c.hashed_world.clone());
            }
        }
        let mut expected = self.expected.lock().expect("expected poisoned");
        Self::prune_expired(&mut expected, now);
        expected.get(token).map(|c| c.hashed_world.clone())
    }

    // Drains and returns the `(token, world)` pairs our client observed that must
    // be echoed to the peer link(s). Called by the orchestration when wiring the
    // echo path. World-attributed so the echo records the mutual-proof half for
    // the correct world.
    pub fn tokens_to_echo_to_peer(&self) -> Vec<(String, String)> {
        let mut to_echo = self.to_echo.lock().expect("to_echo poisoned");
        std::mem::take(&mut *to_echo)
    }

    // Records that we have echoed an observed token (for `hashed_world`) to
    // `peer` over the peer link — completing OUR half of the bidirectional proof
    // for that world (the echo is world-scoped). Called by the
    // orchestration after it sends a `PeerPresenceObserved` to a peer link.
    pub fn record_echoed_to_peer(&self, peer: &str, hashed_world: &str) {
        self.echoed_to_peers
            .lock()
            .expect("echoed_to_peers poisoned")
            .insert((peer.to_string(), hashed_world.to_string()));
    }

    // Whether we have echoed an observed token back to `peer` for `hashed_world`
    // (our half of the mutual proof, world-scoped).
    pub fn echoed_to_peer_for_world(&self, peer: &str, hashed_world: &str) -> bool {
        self.echoed_to_peers
            .lock()
            .expect("echoed_to_peers poisoned")
            .contains(&(peer.to_string(), hashed_world.to_string()))
    }

    // Mutual proof gate. Relaying
    // for `peer`/`world` is only authorized when BOTH: (a) the peer echoed a
    // token WE injected for `world` (`peer_proved_us`), AND (b) we echoed a token
    // the peer injected for `world`, back to that peer
    // (`echoed_to_peer_for_world`). Either half alone — or a half attributed to a
    // DIFFERENT world — is insufficient.
    pub fn is_mutually_proven(&self, peer: &RelayEndpoint, hashed_world: &str) -> bool {
        let key = PeerManager::endpoint_key(peer);
        let peer_proved_us = self
            .proven
            .lock()
            .expect("proven poisoned")
            .contains(&(key.clone(), hashed_world.to_string()));
        let we_proved_peer = self
            .echoed_to_peers
            .lock()
            .expect("echoed_to_peers poisoned")
            .contains(&(key, hashed_world.to_string()));
        peer_proved_us && we_proved_peer
    }

    fn prune_expired(map: &mut HashMap<String, WorldToken>, now: Instant) {
        map.retain(|_, c| c.expires_at > now);
    }
}

impl Default for PresenceProver {
    fn default() -> Self {
        Self::new()
    }
}

// Real `PresenceGate` backing for `PeerManager` (replaces the `AlwaysProven`
// stub). The gate authorizes peering/cert-issuance only on MUTUAL proof:
// the peer echoed a token we injected for the world AND we echoed a token
// the peer injected for the SAME world back to it. Either half alone — or a half
// for a different world — is insufficient. The endpoint is keyed identically to
// `PeerManager` (`host:port`) so the gate lines up with the link table.
// `is_proven` (the single-direction primitive) remains available for the
// orchestration's challenge-needed bookkeeping.
impl PresenceGate for PresenceProver {
    fn is_proven(&self, peer: &RelayEndpoint, hashed_world: &str) -> bool {
        self.is_mutually_proven(peer, hashed_world)
    }
}

impl PresenceProver {
    // Single-direction primitive: the peer echoed a token WE injected for
    // `world`. This is half of the mutual gate (`is_mutually_proven`). Exposed
    // for the orchestration to decide whether a fresh challenge is still needed.
    pub fn peer_proved_us(&self, peer: &RelayEndpoint, hashed_world: &str) -> bool {
        let key = PeerManager::endpoint_key(peer);
        self.proven
            .lock()
            .expect("proven poisoned")
            .contains(&(key, hashed_world.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ep(host: &str, port: u16) -> RelayEndpoint {
        RelayEndpoint {
            host: host.into(),
            port,
            primary: false,
        }
    }

    #[test]
    fn new_challenge_returns_32_char_token() {
        let pp = PresenceProver::new();
        let t = pp.new_challenge("W", Instant::now());
        assert_eq!(t.len(), 32);
    }

    #[test]
    fn new_challenge_tokens_are_unique() {
        let pp = PresenceProver::new();
        let now = Instant::now();
        let a = pp.new_challenge("W", now);
        let b = pp.new_challenge("W", now);
        assert_ne!(a, b);
    }

    #[test]
    fn matches_only_own_outstanding_token() {
        let pp = PresenceProver::new();
        let now = Instant::now();
        let t = pp.new_challenge("W", now);
        assert!(!pp.peer_proved_us(&ep("peerB", 1), "W"));
        pp.record_observed_from_peer("peerB:1", &t, now);
        assert!(pp.peer_proved_us(&ep("peerB", 1), "W"));
        pp.record_observed_from_peer("peerC:1", "garbage", now);
        assert!(!pp.peer_proved_us(&ep("peerC", 1), "W"));
    }

    #[test]
    fn proof_is_scoped_to_the_challenged_world() {
        let pp = PresenceProver::new();
        let now = Instant::now();
        let t = pp.new_challenge("W1", now);
        pp.record_observed_from_peer("peer:1", &t, now);
        assert!(pp.peer_proved_us(&ep("peer", 1), "W1"));
        // a token for W1 does not prove presence in a different world
        assert!(!pp.peer_proved_us(&ep("peer", 1), "W2"));
    }

    #[test]
    fn proof_is_scoped_to_the_echoing_peer() {
        let pp = PresenceProver::new();
        let now = Instant::now();
        let t = pp.new_challenge("W", now);
        pp.record_observed_from_peer("peerA:1", &t, now);
        assert!(pp.peer_proved_us(&ep("peerA", 1), "W"));
        // a different peer that never echoed is not proven
        assert!(!pp.peer_proved_us(&ep("peerB", 1), "W"));
    }

    #[test]
    fn expired_challenge_cannot_be_matched() {
        let pp = PresenceProver::new();
        let t0 = Instant::now();
        let t = pp.new_challenge("W", t0);
        let after = t0 + CHALLENGE_TTL + Duration::from_secs(1);
        pp.record_observed_from_peer("peer:1", &t, after);
        assert!(!pp.peer_proved_us(&ep("peer", 1), "W"));
    }

    #[test]
    fn mutual_gate_requires_both_directions() {
        let pp = PresenceProver::new();
        let now = Instant::now();
        let peer = ep("peer", 9000);
        let key = PeerManager::endpoint_key(&peer);
        let t = pp.new_challenge("W", now);
        // peer echoed our token -> peer proved us, but we have NOT echoed theirs
        pp.record_observed_from_peer(&key, &t, now);
        assert!(pp.peer_proved_us(&peer, "W"));
        assert!(
            !pp.is_mutually_proven(&peer, "W"),
            "one direction alone must not authorize relaying"
        );
        // we echo a token the peer injected for W -> both directions complete
        pp.record_echoed_to_peer(&key, "W");
        assert!(pp.is_mutually_proven(&peer, "W"));
        // and the gate trait now reports proven
        let gate: &dyn PresenceGate = &pp;
        assert!(gate.is_proven(&peer, "W"));
    }

    #[test]
    fn mutual_gate_not_satisfied_by_our_echo_alone() {
        let pp = PresenceProver::new();
        let peer = ep("peer", 9000);
        pp.record_echoed_to_peer(&PeerManager::endpoint_key(&peer), "W");
        // we echoed theirs but the peer never echoed ours
        assert!(!pp.is_mutually_proven(&peer, "W"));
    }

    // The "we proved peer" half must be world-scoped. Echoing a token
    // for world W1 must NOT make us mutually proven for an unrelated world W2,
    // even if the peer proved us in W2.
    #[test]
    fn our_echo_half_is_world_scoped() {
        let pp = PresenceProver::new();
        let now = Instant::now();
        let peer = ep("peer", 9000);
        let key = PeerManager::endpoint_key(&peer);

        // The peer proves us in BOTH W1 and W2 (echoes tokens we injected).
        let t1 = pp.new_challenge("W1", now);
        let t2 = pp.new_challenge("W2", now);
        pp.record_observed_from_peer(&key, &t1, now);
        pp.record_observed_from_peer(&key, &t2, now);
        assert!(pp.peer_proved_us(&peer, "W1"));
        assert!(pp.peer_proved_us(&peer, "W2"));

        // We echo a token the peer injected ONLY for W1.
        pp.record_echoed_to_peer(&key, "W1");

        // Mutually proven for W1 only — NOT for W2 (our echo was W1-attributed).
        assert!(pp.is_mutually_proven(&peer, "W1"));
        assert!(
            !pp.is_mutually_proven(&peer, "W2"),
            "echoing a W1 token must not make us mutually proven for W2"
        );
    }

    #[test]
    fn observed_tokens_are_queued_then_drained_for_echo() {
        let pp = PresenceProver::new();
        let now = Instant::now();
        // both tokens must be known (expected) to be echoed
        pp.expect_observed("tok1", "W", now);
        pp.expect_observed("tok2", "W", now);
        pp.on_client_observed("tok1", now);
        pp.on_client_observed("tok2", now);
        let drained = pp.tokens_to_echo_to_peer();
        assert_eq!(
            drained,
            vec![
                ("tok1".to_string(), "W".to_string()),
                ("tok2".to_string(), "W".to_string())
            ]
        );
        // drained once; the queue is now empty
        assert!(pp.tokens_to_echo_to_peer().is_empty());
    }

    // An observed token that matches NO outstanding/expected
    // challenge (e.g. an attacker-injected `!bvcp <garbage>` string) is neither
    // echoed to a peer nor able to advance proof state.
    #[test]
    fn unknown_observed_token_is_neither_echoed_nor_advances_proof() {
        let pp = PresenceProver::new();
        let now = Instant::now();
        // no expect_observed / new_challenge for this token
        pp.on_client_observed("attacker-injected-garbage", now);
        assert!(
            pp.tokens_to_echo_to_peer().is_empty(),
            "an unknown observed token must not be echoed to any peer"
        );
        // and since nothing was echoed, no peer can become mutually proven via it
        let peer = ep("attacker", 9000);
        assert!(!pp.echoed_to_peer_for_world(&PeerManager::endpoint_key(&peer), "W"));
        assert!(!pp.is_mutually_proven(&peer, "W"));
    }

    #[test]
    fn known_outstanding_token_can_be_echoed() {
        let pp = PresenceProver::new();
        let now = Instant::now();
        // a token we ourselves injected is a "known" challenge
        let t = pp.new_challenge("W", now);
        pp.on_client_observed(&t, now);
        let drained = pp.tokens_to_echo_to_peer();
        assert_eq!(drained, vec![(t, "W".to_string())]);
    }

    // Security invariant: the API offers no way to push a CHALLENGE token onto a
    // peer link unless our own client genuinely observed it back. A freshly
    // generated challenge token is NOT present in the echo queue until observed,
    // and only a KNOWN (outstanding/expected) observed token is ever queued.
    #[test]
    fn challenge_tokens_never_enter_the_peer_echo_path_unobserved() {
        let pp = PresenceProver::new();
        let now = Instant::now();
        let challenge = pp.new_challenge("W", now);
        // generating a challenge must not place anything on the peer-echo path
        assert!(
            pp.tokens_to_echo_to_peer().is_empty(),
            "a challenge must never be queued toward a peer link before it is observed"
        );
        // an observed token for the realm (expected) reaches the echo path
        pp.expect_observed("observed-from-realm", "W", now);
        pp.on_client_observed("observed-from-realm", now);
        let echoed = pp.tokens_to_echo_to_peer();
        assert!(
            !echoed.iter().any(|(t, _)| t == &challenge),
            "the challenge token must never be echoed to a peer unless observed"
        );
        assert_eq!(echoed, vec![("observed-from-realm".to_string(), "W".to_string())]);
    }

    #[test]
    fn implements_presence_gate() {
        let pp = PresenceProver::new();
        let now = Instant::now();
        let peer = ep("peer", 9000);
        let key = PeerManager::endpoint_key(&peer);
        let t = pp.new_challenge("W", now);
        let gate: &dyn PresenceGate = &pp;
        assert!(!gate.is_proven(&peer, "W"));
        // the gate requires MUTUAL proof: both the peer echoing our token and us
        // echoing theirs for the SAME world.
        pp.record_observed_from_peer(&key, &t, now);
        assert!(!gate.is_proven(&peer, "W"));
        pp.record_echoed_to_peer(&key, "W");
        assert!(gate.is_proven(&peer, "W"));
    }
}
