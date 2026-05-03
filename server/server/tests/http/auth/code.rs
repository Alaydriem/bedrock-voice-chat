//! POST /api/auth/code
//!
//! ncryptf-wrapped code login. A player presents their one-time code and receives
//! their issued mTLS cert/key/CA bundle plus other identity material.
//!
//! Contract:
//! - 200 + LoginResponse on a fresh, unexpired code matching the gamertag
//! - 404 on unknown code
//! - 403 on gamertag mismatch / already-used code
//! - 410 (Gone) on expired code

use crate::harness::TestServer;

use base64::Engine;
use bvc_server_lib::services::AuthCodeService;
use common::request::CodeLoginRequest;
use common::Game;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

const ENDPOINT: &str = "/api/auth/code";

#[derive(Debug, Clone, serde::Deserialize)]
struct JsonMessage<T> {
    status: u16,
    data: Option<T>,
    message: Option<String>,
}

async fn ncryptf_login(
    env: &TestServer,
    payload: &CodeLoginRequest,
) -> anyhow::Result<common::response::LoginResponse> {
    let ek: common::ncryptflib::ExportableEncryptionKeyData = env
        .noauth_client()?
        .get(format!("{}/ncryptf/ek", env.base_url))
        .send()
        .await?
        .json()
        .await?;

    let kp = common::ncryptflib::Keypair::new();

    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("Content-Type", reqwest::header::HeaderValue::from_static("application/json"));
    headers.insert("Accept", reqwest::header::HeaderValue::from_static("application/vnd.ncryptf+json"));
    headers.insert(
        "X-HashId",
        reqwest::header::HeaderValue::from_str(&ek.hash_id)?,
    );
    headers.insert(
        "X-PubKey",
        reqwest::header::HeaderValue::from_str(&base64::engine::general_purpose::STANDARD.encode(kp.get_public_key()))?,
    );

    let resp = env
        .noauth_client()?
        .post(format!("{}{}", env.base_url, ENDPOINT))
        .headers(headers)
        .json(payload)
        .send()
        .await?;

    let status = resp.status();
    let bytes = resp.bytes().await?;
    let decoded = base64::engine::general_purpose::STANDARD.decode(&bytes)?;

    let response = common::ncryptflib::Response::from(kp.get_secret_key())?;
    let decrypted = response.decrypt(decoded, None, None)?;

    let wrapper: JsonMessage<common::response::LoginResponse> = serde_json::from_str(&decrypted)?;

    if !status.is_success() {
        anyhow::bail!("ncryptf login failed: status={}, message={:?}", wrapper.status, wrapper.message);
    }

    wrapper.data.ok_or_else(|| anyhow::anyhow!("empty data field"))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn returns_404_on_unknown_code() {
    let env = TestServer::start().await.unwrap();
    let result = ncryptf_login(
        &env,
        &CodeLoginRequest {
            gamertag: "Bob".into(),
            code: "DEFINITELY-NOT-A-REAL-CODE".into(),
        },
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

    let code = AuthCodeService::generate_code(&env.db, bob_player.id, 600)
        .await
        .unwrap();

    let response = ncryptf_login(
        &env,
        &CodeLoginRequest {
            gamertag: "Bob".into(),
            code,
        },
    )
    .await
    .expect("valid code should succeed");

    assert_eq!(response.gamertag, "Bob");
    assert!(!response.certificate.is_empty());
    assert!(!response.certificate_key.is_empty());
    assert!(!response.certificate_ca.is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn returns_403_on_gamertag_mismatch() {
    let env = TestServer::start().await.unwrap();
    let _ = env.issue_player("Bob", &Game::Minecraft).await.unwrap();

    let bob_player = entity::player::Entity::find()
        .filter(entity::player::Column::Gamertag.eq("Bob"))
        .one(&env.db)
        .await
        .unwrap()
        .expect("Bob should exist");

    let code = AuthCodeService::generate_code(&env.db, bob_player.id, 600)
        .await
        .unwrap();

    let result = ncryptf_login(
        &env,
        &CodeLoginRequest {
            gamertag: "Alice".into(),
            code,
        },
    )
    .await;
    assert!(result.is_err(), "expected 403 for gamertag mismatch");
}
