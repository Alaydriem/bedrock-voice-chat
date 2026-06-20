//! POST /api/relay/peer-link — Flow 2 (designated-peer / operator API).
//!
//! mTLS-gated: a client presenting a CA-signed cert gets a scoped, single-use,
//! recipient-bound peer-link code directly in the response; it then redeems the
//! code at /api/relay/peer-redeem for a `server::`-CN peer cert. An unauthenticated
//! caller (no client cert) cannot reach the route.

use crate::harness::TestServer;

use common::Game;
use common::structs::relay::{
    PeerCertResponse, PeerLinkRequest, PeerLinkResponse, PeerRedeemRequest,
};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn peer_link_grants_a_scoped_code_an_mtls_client_can_redeem() {
    let env = TestServer::start_with_relay(true).await.unwrap();

    // A designated peer: an mTLS player holding the `peer_link` permission.
    let (cert, key) = env
        .issue_player_with_perm("designated-peer", &Game::Minecraft, "peer_link")
        .await
        .unwrap();
    let client = env.mtls_client(&cert, &key).unwrap();

    let link_body = PeerLinkRequest {
        hashed_world: "hW".to_string(),
        recipient_host: "peer.example.com".to_string(),
        recipient_port: 7000,
    };
    let resp = client
        .post(format!("{}/api/relay/peer-link", env.base_url))
        .json(&link_body)
        .send()
        .await
        .unwrap();
    assert!(
        resp.status().is_success(),
        "granted mTLS client gets a peer-link code, status {}",
        resp.status()
    );
    let code = resp.json::<PeerLinkResponse>().await.unwrap().code;
    assert!(!code.is_empty(), "a non-empty code is issued");

    // The code redeems for the bound recipient into a `server::`-CN peer cert.
    let redeem_body = PeerRedeemRequest {
        code: code.clone(),
        presenter_host: "peer.example.com".to_string(),
        presenter_port: 7000,
    };
    let redeem: PeerCertResponse = client
        .post(format!("{}/api/relay/peer-redeem", env.base_url))
        .json(&redeem_body)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(redeem.cert_pem.contains("BEGIN CERTIFICATE"));
    assert!(redeem.key_pem.contains("PRIVATE KEY"));

    // Single-use: a second redemption of the same code is refused.
    let resp2 = client
        .post(format!("{}/api/relay/peer-redeem", env.base_url))
        .json(&redeem_body)
        .send()
        .await
        .unwrap();
    assert!(
        !resp2.status().is_success(),
        "a peer-link code is single-use; second redemption must be refused"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn peer_link_rejects_an_authenticated_player_without_the_permission() {
    let env = TestServer::start_with_relay(true).await.unwrap();
    // A valid mTLS player, but WITHOUT the `peer_link` permission.
    let (cert, key) = env
        .issue_player("ordinary-player", &Game::Minecraft)
        .await
        .unwrap();
    let client = env.mtls_client(&cert, &key).unwrap();

    let link_body = PeerLinkRequest {
        hashed_world: "hW".to_string(),
        recipient_host: "peer.example.com".to_string(),
        recipient_port: 7000,
    };
    let resp = client
        .post(format!("{}/api/relay/peer-link", env.base_url))
        .json(&link_body)
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        reqwest::StatusCode::FORBIDDEN,
        "an authenticated player without the peer_link permission must be refused"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn peer_link_rejects_an_unauthenticated_caller() {
    let env = TestServer::start_with_relay(true).await.unwrap();
    // No client certificate presented: the mTLS guard fails closed.
    let client = env.noauth_client().unwrap();

    let link_body = PeerLinkRequest {
        hashed_world: "hW".to_string(),
        recipient_host: "peer.example.com".to_string(),
        recipient_port: 7000,
    };
    let resp = client
        .post(format!("{}/api/relay/peer-link", env.base_url))
        .json(&link_body)
        .send()
        .await
        .unwrap();
    assert!(
        !resp.status().is_success(),
        "an unauthenticated caller must not obtain a peer-link code, got {}",
        resp.status()
    );
}
