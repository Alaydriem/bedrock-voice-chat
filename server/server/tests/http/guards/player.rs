//! `PlayerGuard`
//!
//! Contract:
//! - resolves the certificate to a player, the way every player-facing route needs
//! - refuses a certificate whose fingerprint has been revoked, even though the certificate
//!   is still cryptographically valid and unexpired
//! - one player's revocation does not touch another's certificate

use common::Game;

use crate::harness::http_client::MtlsClient;
use crate::harness::{HttpAssert, TestServer};

const ENDPOINT: &str = "/api/channel";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_revoked_certificate_is_refused() {
    let env = TestServer::start().await.unwrap();
    let (cert, key) = env.issue_player("Revoked", &Game::Minecraft).await.unwrap();

    let before = MtlsClient::with_identity(&env.ca_pem, &cert, &key)
        .unwrap()
        .get(format!("{}{}", env.base_url, ENDPOINT))
        .send()
        .await
        .unwrap();
    HttpAssert::status(before.status().as_u16(), 200);

    env.revoke_certificate(&cert).await.unwrap();

    let after = MtlsClient::with_identity(&env.ca_pem, &cert, &key)
        .unwrap()
        .get(format!("{}{}", env.base_url, ENDPOINT))
        .send()
        .await
        .unwrap();
    HttpAssert::status(after.status().as_u16(), 403);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_unrelated_revocation_does_not_affect_a_valid_certificate() {
    let env = TestServer::start().await.unwrap();
    let (victim_cert, _) = env.issue_player("Victim", &Game::Minecraft).await.unwrap();
    let (cert, key) = env
        .issue_player("Bystander", &Game::Minecraft)
        .await
        .unwrap();

    env.revoke_certificate(&victim_cert).await.unwrap();

    let response = MtlsClient::with_identity(&env.ca_pem, &cert, &key)
        .unwrap()
        .get(format!("{}{}", env.base_url, ENDPOINT))
        .send()
        .await
        .unwrap();
    HttpAssert::status(response.status().as_u16(), 200);
}

// Banning acted only on the next login before this: a banished player who already held a
// certificate kept full access for its whole life. The check lives in the guard rather than
// in each route so it cannot be omitted by the next route somebody adds.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_banished_player_is_refused() {
    let env = TestServer::start().await.unwrap();
    let (cert, key) = env.issue_player("Banished", &Game::Minecraft).await.unwrap();

    let before = MtlsClient::with_identity(&env.ca_pem, &cert, &key)
        .unwrap()
        .get(format!("{}{}", env.base_url, ENDPOINT))
        .send()
        .await
        .unwrap();
    HttpAssert::status(before.status().as_u16(), 200);

    env.mark_banished("Banished", &Game::Minecraft, true)
        .await
        .unwrap();

    let after = MtlsClient::with_identity(&env.ca_pem, &cert, &key)
        .unwrap()
        .get(format!("{}{}", env.base_url, ENDPOINT))
        .send()
        .await
        .unwrap();
    HttpAssert::status(after.status().as_u16(), 403);
}

// Introspect is behind the same guard as everything else. An earlier note in the introspect
// tests argued a banished player should still be able to read their own state to learn why
// they were locked out; the operator's decision is that they cannot.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_banished_player_cannot_introspect() {
    let env = TestServer::start().await.unwrap();
    let (cert, key) = env.issue_player("Banished", &Game::Minecraft).await.unwrap();
    env.mark_banished("Banished", &Game::Minecraft, true)
        .await
        .unwrap();

    let response = MtlsClient::with_identity(&env.ca_pem, &cert, &key)
        .unwrap()
        .get(format!("{}/api/auth/introspect", env.base_url))
        .send()
        .await
        .unwrap();

    HttpAssert::status(response.status().as_u16(), 403);
}

// A certificate this CA signed for a name with no player row must not reach a route. The
// channel routes previously read the CN straight off the certificate and never asked the
// database anything, so nothing on that path had ever checked this.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_certificate_with_no_player_row_is_refused() {
    let env = TestServer::start().await.unwrap();
    let (cert, key) = env
        .cert_service
        .sign_player_cert("NeverRegistered", &Game::Minecraft)
        .unwrap();

    let response = MtlsClient::with_identity(&env.ca_pem, &cert.pem(), &key.serialize_pem())
        .unwrap()
        .get(format!("{}{}", env.base_url, ENDPOINT))
        .send()
        .await
        .unwrap();

    HttpAssert::status(response.status().as_u16(), 403);
}
