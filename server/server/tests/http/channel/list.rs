//! `GET /api/channel`
//!
//! Contract:
//! - a game server presenting its access token may read the list, which is what the
//!   in-game panel's group page needs
//! - an authenticated player may still read it, unchanged
//! - a request carrying neither credential is refused

use common::Game;

use crate::harness::http_client::MtlsClient;
use crate::harness::{HttpAssert, TestServer};

const ENDPOINT: &str = "/api/channel";
const TOKEN: &str = "test-mc-token";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_game_access_token_may_list_channels() {
    let env = TestServer::start().await.unwrap();

    let resp = env
        .noauth_client()
        .unwrap()
        .get(format!("{}{}", env.base_url, ENDPOINT))
        .header("Authorization", format!("Bearer {TOKEN}"))
        .send()
        .await
        .unwrap();

    HttpAssert::status(resp.status().as_u16(), 200);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_player_certificate_may_still_list_channels() {
    let env = TestServer::start().await.unwrap();
    let (cert, key) = env.issue_player("Reader", &Game::Minecraft).await.unwrap();

    let resp = MtlsClient::with_identity(&env.ca_pem, &cert, &key)
        .unwrap()
        .get(format!("{}{}", env.base_url, ENDPOINT))
        .send()
        .await
        .unwrap();

    HttpAssert::status(resp.status().as_u16(), 200);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_request_with_no_credential_is_refused() {
    let env = TestServer::start().await.unwrap();

    let resp = env
        .noauth_client()
        .unwrap()
        .get(format!("{}{}", env.base_url, ENDPOINT))
        .send()
        .await
        .unwrap();

    assert_ne!(
        resp.status().as_u16(),
        200,
        "listing channels without any credential must be refused"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_revoked_certificate_is_refused_even_when_a_token_is_also_presented() {
    let env = TestServer::start().await.unwrap();
    let (cert, key) = env.issue_player("Revoked", &Game::Minecraft).await.unwrap();
    env.revoke_certificate(&cert).await.unwrap();

    let resp = MtlsClient::with_identity(&env.ca_pem, &cert, &key)
        .unwrap()
        .get(format!("{}{}", env.base_url, ENDPOINT))
        .header("Authorization", format!("Bearer {TOKEN}"))
        .send()
        .await
        .unwrap();

    HttpAssert::status(resp.status().as_u16(), 403);
}
