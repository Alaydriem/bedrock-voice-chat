//! POST /api/auth/code
//!
//! ncryptf-wrapped code login. A player presents their one-time code and receives
//! their issued mTLS cert/key/CA bundle plus other identity material.
//!
//! The code is the whole credential. It resolves the player, so the response also
//! reports the gamertag and the game rather than the caller asserting them.
//!
//! Contract:
//! - 200 + LoginResponse on a fresh, unexpired code
//! - 404 on unknown code
//! - 403 on an already-used ephemeral code
//! - 410 (Gone) on expired code

use crate::harness::{NcryptfLogin, TestServer};

use bvc_server_lib::services::AuthCodeService;
use common::Game;
use common::request::CodeLoginRequest;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn returns_404_on_unknown_code() {
    let env = TestServer::start().await.unwrap();
    let result = NcryptfLogin::perform(
        &env,
        &CodeLoginRequest { code: "DEFINITELY-NOT-A-REAL-CODE".into() },
    )
    .await;
    assert!(result.is_err(), "expected 404 for unknown code");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn returns_login_response_on_valid_code() {
    let env = TestServer::start().await.unwrap();
    let _ = env.issue_player("Bob", &Game::Minecraft).await.unwrap();

    let bob_player = entity::player::Entity::find()
        .filter(entity::player::Column::Gamertag.eq("Bob"))
        .one(&env.db)
        .await
        .unwrap()
        .expect("Bob should exist");

    let code = AuthCodeService::generate_code(&env.db, bob_player.id, 600, true)
        .await
        .unwrap();

    let response = NcryptfLogin::perform(
        &env,
        &CodeLoginRequest { code },
    )
    .await
    .expect("valid code should succeed");

    assert_eq!(response.gamertag, "Bob");
    assert!(!response.certificate.is_empty());
    assert!(!response.certificate_key.is_empty());
    assert!(!response.certificate_ca.is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rejects_reused_code() {
    let env = TestServer::start().await.unwrap();
    let _ = env.issue_player("Bob", &Game::Minecraft).await.unwrap();

    let bob_player = entity::player::Entity::find()
        .filter(entity::player::Column::Gamertag.eq("Bob"))
        .one(&env.db)
        .await
        .unwrap()
        .expect("Bob should exist");

    let code = AuthCodeService::generate_code(&env.db, bob_player.id, 600, true)
        .await
        .unwrap();

    // First redemption of a fresh code succeeds.
    let first = NcryptfLogin::perform(
        &env,
        &CodeLoginRequest { code: code.clone() },
    )
    .await;
    assert!(
        first.is_ok(),
        "first redemption of a fresh code must succeed"
    );

    // A code is single-use: the same code must not redeem a second time.
    let second = NcryptfLogin::perform(
        &env,
        &CodeLoginRequest { code },
    )
    .await;
    assert!(second.is_err(), "an already-used code must be rejected");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn non_ephemeral_code_redeems_more_than_once() {
    let env = TestServer::start().await.unwrap();
    let _ = env.issue_player("Bob", &Game::Minecraft).await.unwrap();

    let bob_player = entity::player::Entity::find()
        .filter(entity::player::Column::Gamertag.eq("Bob"))
        .one(&env.db)
        .await
        .unwrap()
        .expect("Bob should exist");

    // A non-ephemeral code is reusable until it expires.
    let code = AuthCodeService::generate_code(&env.db, bob_player.id, 600, false)
        .await
        .unwrap();

    let first = NcryptfLogin::perform(
        &env,
        &CodeLoginRequest { code: code.clone() },
    )
    .await;
    assert!(
        first.is_ok(),
        "first redemption of a reusable code must succeed"
    );

    let second = NcryptfLogin::perform(
        &env,
        &CodeLoginRequest { code },
    )
    .await;
    assert!(
        second.is_ok(),
        "a non-ephemeral code must redeem again before it expires"
    );
}

/// The code alone resolves who the caller is.
///
/// A client sends only a code, so the response is its only source for both fields. If
/// either stopped being reported the CLI would store an identity under the wrong game,
/// and the desktop client would have to ask the user and believe the answer.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn resolves_the_player_and_game_from_the_code_alone() {
    let env = TestServer::start().await.unwrap();
    let _ = env.issue_player("Bob", &Game::Minecraft).await.unwrap();

    let bob_player = entity::player::Entity::find()
        .filter(entity::player::Column::Gamertag.eq("Bob"))
        .one(&env.db)
        .await
        .unwrap()
        .expect("Bob should exist");

    let code = AuthCodeService::generate_code(&env.db, bob_player.id, 600, true)
        .await
        .unwrap();

    let response = NcryptfLogin::perform(&env, &CodeLoginRequest { code })
        .await
        .expect("a code identifies its player without a gamertag");

    assert_eq!(response.gamertag, "Bob");
    assert_eq!(response.game, Some(Game::Minecraft));
}
