//! PATCH /api/admin/user/banish
//!
//! Contract:
//! - 401 / 403 / banished — same admin-gate as every admin route
//! - 200 toggling banished on another player
//! - 404 when the target gamertag/game pair doesn't exist
//! - 409 when admin tries to banish themselves

use crate::harness::http_client::MtlsClient;
use crate::harness::server::{ADMIN_GAME, ADMIN_GAMERTAG};
use crate::harness::{HttpAssert, TestServer};

use common::Game;
use common::request::admin::BanishUserRequest;

const ENDPOINT: &str = "/api/admin/user/banish";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn returns_401_without_client_cert() {
    let env = TestServer::start().await.unwrap();
    let resp = env
        .noauth_client()
        .unwrap()
        .patch(format!("{}{}", env.base_url, ENDPOINT))
        .json(&BanishUserRequest {
            gamertag: "Bob".into(),
            game: Game::Minecraft,
            banish: true,
        })
        .send()
        .await
        .unwrap();
    HttpAssert::status(resp.status().as_u16(), 401);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn returns_403_for_non_admin() {
    let env = TestServer::start().await.unwrap();
    let (cert, key) = env.issue_player("Bob", &Game::Minecraft).await.unwrap();
    let resp = MtlsClient::with_identity(&env.ca_pem, &cert, &key)
        .unwrap()
        .patch(format!("{}{}", env.base_url, ENDPOINT))
        .json(&BanishUserRequest {
            gamertag: "Bob".into(),
            game: Game::Minecraft,
            banish: true,
        })
        .send()
        .await
        .unwrap();
    HttpAssert::status(resp.status().as_u16(), 403);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn returns_404_when_target_missing() {
    let env = TestServer::start().await.unwrap();
    let resp = env
        .admin_client()
        .unwrap()
        .patch(format!("{}{}", env.base_url, ENDPOINT))
        .json(&BanishUserRequest {
            gamertag: "Ghost".into(),
            game: Game::Minecraft,
            banish: true,
        })
        .send()
        .await
        .unwrap();
    HttpAssert::status(resp.status().as_u16(), 404);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn admin_can_banish_other_player() {
    let env = TestServer::start().await.unwrap();
    let _ = env.issue_player("Bob", &Game::Minecraft).await.unwrap();
    let resp = env
        .admin_client()
        .unwrap()
        .patch(format!("{}{}", env.base_url, ENDPOINT))
        .json(&BanishUserRequest {
            gamertag: "Bob".into(),
            game: Game::Minecraft,
            banish: true,
        })
        .send()
        .await
        .unwrap();
    HttpAssert::status(resp.status().as_u16(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["banished"].as_bool().unwrap(), true);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn returns_403_for_banished_admin() {
    let env = TestServer::start().await.unwrap();
    env.mark_banished(ADMIN_GAMERTAG, &ADMIN_GAME, true)
        .await
        .unwrap();

    let _ = env.issue_player("Bob", &Game::Minecraft).await.unwrap();
    let resp = env
        .admin_client()
        .unwrap()
        .patch(format!("{}{}", env.base_url, ENDPOINT))
        .json(&BanishUserRequest {
            gamertag: "Bob".into(),
            game: Game::Minecraft,
            banish: true,
        })
        .send()
        .await
        .unwrap();
    HttpAssert::status(resp.status().as_u16(), 403);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn admin_can_unbanish_player() {
    let env = TestServer::start().await.unwrap();
    let _ = env.issue_player("Bob", &Game::Minecraft).await.unwrap();
    let client = env.admin_client().unwrap();

    // Banish first
    let banish = client
        .patch(format!("{}{}", env.base_url, ENDPOINT))
        .json(&BanishUserRequest {
            gamertag: "Bob".into(),
            game: Game::Minecraft,
            banish: true,
        })
        .send()
        .await
        .unwrap();
    HttpAssert::status(banish.status().as_u16(), 200);

    // Then unbanish
    let unbanish = client
        .patch(format!("{}{}", env.base_url, ENDPOINT))
        .json(&BanishUserRequest {
            gamertag: "Bob".into(),
            game: Game::Minecraft,
            banish: false,
        })
        .send()
        .await
        .unwrap();
    HttpAssert::status(unbanish.status().as_u16(), 200);
    let body: serde_json::Value = unbanish.json().await.unwrap();
    assert_eq!(body["banished"].as_bool().unwrap(), false);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rejects_self_banish() {
    let env = TestServer::start().await.unwrap();
    let resp = env
        .admin_client()
        .unwrap()
        .patch(format!("{}{}", env.base_url, ENDPOINT))
        .json(&BanishUserRequest {
            gamertag: ADMIN_GAMERTAG.into(),
            game: ADMIN_GAME.clone(),
            banish: true,
        })
        .send()
        .await
        .unwrap();
    HttpAssert::status(resp.status().as_u16(), 409);
}

// Banning has to act on the certificate the player already holds. Setting the flag alone
// took effect only at their next login, which a banned player has no reason to perform.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn banning_revokes_the_players_current_certificate() {
    let env = TestServer::start().await.unwrap();
    let (cert, key) = env.issue_player("Target", &Game::Minecraft).await.unwrap();

    let before = MtlsClient::with_identity(&env.ca_pem, &cert, &key)
        .unwrap()
        .get(format!("{}/api/channel", env.base_url))
        .send()
        .await
        .unwrap();
    HttpAssert::status(before.status().as_u16(), 200);

    let banish = env
        .admin_client()
        .unwrap()
        .patch(format!("{}{}", env.base_url, ENDPOINT))
        .json(&BanishUserRequest {
            gamertag: "Target".into(),
            game: Game::Minecraft,
            banish: true,
        })
        .send()
        .await
        .unwrap();
    HttpAssert::status(banish.status().as_u16(), 200);

    let after = MtlsClient::with_identity(&env.ca_pem, &cert, &key)
        .unwrap()
        .get(format!("{}/api/channel", env.base_url))
        .send()
        .await
        .unwrap();
    HttpAssert::status(after.status().as_u16(), 403);
}

// Unbanning does not un-revoke: the certificate is gone for good. The player logs in and is
// issued a new one, which `banished` no longer blocks.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unbanning_does_not_restore_the_revoked_certificate() {
    let env = TestServer::start().await.unwrap();
    let (cert, key) = env.issue_player("Target", &Game::Minecraft).await.unwrap();

    for banish in [true, false] {
        let resp = env
            .admin_client()
            .unwrap()
            .patch(format!("{}{}", env.base_url, ENDPOINT))
            .json(&BanishUserRequest {
                gamertag: "Target".into(),
                game: Game::Minecraft,
                banish,
            })
            .send()
            .await
            .unwrap();
        HttpAssert::status(resp.status().as_u16(), 200);
    }

    let after = MtlsClient::with_identity(&env.ca_pem, &cert, &key)
        .unwrap()
        .get(format!("{}/api/channel", env.base_url))
        .send()
        .await
        .unwrap();
    HttpAssert::status(after.status().as_u16(), 403);
}
