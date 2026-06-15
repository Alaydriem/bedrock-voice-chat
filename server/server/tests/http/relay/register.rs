//! POST /relay/challenge, /relay/register and /relay/lookup — discovery plane.
//!
//! Public TLS (no client-cert guard); the caller authenticates the relay via SPKI
//! pinning. Routes are mounted only when `features.relay.enabled`. Registration is
//! gated on an endpoint-control-proven token: the harness injects a
//! permissive reachability stub so the register/lookup mechanics are exercised;
//! the deny path is unit-tested in `registry.rs`.

use crate::harness::TestServer;

use common::structs::relay::{
    LookupRequest, LookupResponse, RegisterChallengeRequest, RegisterChallengeResponse,
    RegisterRequest, RelayEndpoint,
};

fn ep(host: &str, port: u16) -> RelayEndpoint {
    RelayEndpoint {
        host: host.to_string(),
        port,
        primary: false,
    }
}

// Obtains an endpoint-control token via the challenge route.
async fn challenge_token(
    client: &reqwest::Client,
    base_url: &str,
    endpoint: &RelayEndpoint,
) -> String {
    let body = RegisterChallengeRequest {
        endpoint: endpoint.clone(),
    };
    let resp: RegisterChallengeResponse = client
        .post(format!("{}/relay/challenge", base_url))
        .json(&body)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    resp.token
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn register_then_lookup_scoped_and_self_excluded() {
    let env = TestServer::start_with_relay(true).await.unwrap();
    let client = env.noauth_client().unwrap();

    let a = ep("a.example.com", 1);
    let b = ep("b.example.com", 2);

    // Keep `a`'s endpoint-control token so the gated lookup can present it; the
    // relay confirms the caller controls the endpoint it claims.
    let mut a_token = String::new();
    for endpoint in [a.clone(), b.clone()] {
        let token = challenge_token(&client, &env.base_url, &endpoint).await;
        if endpoint == a {
            a_token = token.clone();
        }
        let body = RegisterRequest {
            hashed_world: "hW".to_string(),
            endpoint,
            ttl_secs: 60,
            token,
        };
        let resp = client
            .post(format!("{}/relay/register", env.base_url))
            .json(&body)
            .send()
            .await
            .unwrap();
        assert!(resp.status().is_success(), "register status {}", resp.status());
    }

    let lookup_body = LookupRequest {
        caller: a.clone(),
        hashed_worlds: vec!["hW".to_string()],
        token: a_token,
    };
    let resp: LookupResponse = client
        .post(format!("{}/relay/lookup", env.base_url))
        .json(&lookup_body)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(resp.worlds.get("hW").unwrap(), &vec![b.clone()]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unregistered_caller_gets_empty() {
    let env = TestServer::start_with_relay(true).await.unwrap();
    let client = env.noauth_client().unwrap();

    let a = ep("a.example.com", 1);
    let token = challenge_token(&client, &env.base_url, &a).await;
    let body = RegisterRequest {
        hashed_world: "hW".to_string(),
        endpoint: a.clone(),
        ttl_secs: 60,
        token,
    };
    client
        .post(format!("{}/relay/register", env.base_url))
        .json(&body)
        .send()
        .await
        .unwrap();

    // The outsider proves control of its OWN endpoint (so the lookup token gate
    // passes), but it never registered in the world — so it sees no peers.
    let outsider = ep("outsider.example.com", 99);
    let outsider_token = challenge_token(&client, &env.base_url, &outsider).await;
    let lookup_body = LookupRequest {
        caller: outsider,
        hashed_worlds: vec!["hW".to_string()],
        token: outsider_token,
    };
    let resp: LookupResponse = client
        .post(format!("{}/relay/lookup", env.base_url))
        .json(&lookup_body)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert!(resp.worlds.get("hW").map(|v| v.is_empty()).unwrap_or(true));
}

// A register with NO endpoint-control token is rejected (default deny), so an
// attacker who merely knows the world hash cannot inject an endpoint and then
// enumerate peers via lookup.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn register_without_token_is_rejected() {
    let env = TestServer::start_with_relay(true).await.unwrap();
    let client = env.noauth_client().unwrap();

    let attacker = ep("attacker.example.com", 6666);
    let body = RegisterRequest {
        hashed_world: "hW".to_string(),
        endpoint: attacker.clone(),
        ttl_secs: 60,
        token: String::new(),
    };
    let resp = client
        .post(format!("{}/relay/register", env.base_url))
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        reqwest::StatusCode::FORBIDDEN,
        "register without a proven endpoint token must be denied"
    );

    // And since it never registered (and presents no proven token), it sees no
    // peers in the world — the lookup token gate also denies it.
    let lookup_body = LookupRequest {
        caller: attacker,
        hashed_worlds: vec!["hW".to_string()],
        token: String::new(),
    };
    let resp: LookupResponse = client
        .post(format!("{}/relay/lookup", env.base_url))
        .json(&lookup_body)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(resp.worlds.get("hW").map(|v| v.is_empty()).unwrap_or(true));
}

// Lookup is gated on the same endpoint-control proof as register. An attacker
// who knows the world hash AND a registered member's
// endpoint cannot pass `caller = victim_endpoint` to enumerate: it has no token
// proving control of the victim's endpoint. With a token proven for its OWN
// endpoint (mismatched to the claimed caller) it is still denied; only the member
// presenting its own proven token enumerates the scoped, self-excluded peers.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn lookup_requires_endpoint_control_token() {
    let env = TestServer::start_with_relay(true).await.unwrap();
    let client = env.noauth_client().unwrap();

    let victim = ep("victim.example.com", 1);
    let member = ep("member.example.com", 2);

    // Register two real members of the world.
    let mut victim_token = String::new();
    for endpoint in [victim.clone(), member.clone()] {
        let token = challenge_token(&client, &env.base_url, &endpoint).await;
        if endpoint == victim {
            victim_token = token.clone();
        }
        let body = RegisterRequest {
            hashed_world: "hW".to_string(),
            endpoint,
            ttl_secs: 60,
            token,
        };
        let resp = client
            .post(format!("{}/relay/register", env.base_url))
            .json(&body)
            .send()
            .await
            .unwrap();
        assert!(resp.status().is_success());
    }

    // Attacker knows the world hash AND the victim's endpoint, but controls only
    // its own. Claiming `caller = victim` with a token proven for the attacker's
    // endpoint is denied (token not bound to the claimed caller).
    let attacker = ep("attacker.example.com", 6666);
    let attacker_token = challenge_token(&client, &env.base_url, &attacker).await;
    let spoof_body = LookupRequest {
        caller: victim.clone(),
        hashed_worlds: vec!["hW".to_string()],
        token: attacker_token,
    };
    let resp: LookupResponse = client
        .post(format!("{}/relay/lookup", env.base_url))
        .json(&spoof_body)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(
        resp.worlds.get("hW").map(|v| v.is_empty()).unwrap_or(true),
        "a token proven for one endpoint must not authorize lookup as another"
    );

    // Claiming `caller = victim` with NO token is also denied.
    let no_token = LookupRequest {
        caller: victim.clone(),
        hashed_worlds: vec!["hW".to_string()],
        token: String::new(),
    };
    let resp: LookupResponse = client
        .post(format!("{}/relay/lookup", env.base_url))
        .json(&no_token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(resp.worlds.get("hW").map(|v| v.is_empty()).unwrap_or(true));

    // The victim itself, presenting its own proven token, enumerates peers
    // (self-excluded).
    let valid = LookupRequest {
        caller: victim.clone(),
        hashed_worlds: vec!["hW".to_string()],
        token: victim_token,
    };
    let resp: LookupResponse = client
        .post(format!("{}/relay/lookup", env.base_url))
        .json(&valid)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(resp.worlds.get("hW").unwrap(), &vec![member.clone()]);
}
