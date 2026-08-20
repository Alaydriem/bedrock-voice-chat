//! GET /api/admin/user
//!
//! Contract:
//! - 401 without a client certificate, 403 for a non-admin: the shared admin gate
//! - 200 with a page of rows an operator can act on
//! - `search` and `game` narrow the page, and `total` counts the filtered set
//! - a banished player is listed, with the flag set
//! - `page_size` is clamped, so one request cannot pull the whole table

use crate::harness::http_client::MtlsClient;
use crate::harness::{HttpAssert, TestServer};

use common::Game;

const ENDPOINT: &str = "/api/admin/user";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn returns_401_without_client_cert() {
    let env = TestServer::start().await.unwrap();
    let resp = env
        .noauth_client()
        .unwrap()
        .get(format!("{}{}", env.base_url, ENDPOINT))
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
        .get(format!("{}{}", env.base_url, ENDPOINT))
        .send()
        .await
        .unwrap();
    HttpAssert::status(resp.status().as_u16(), 403);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn lists_registered_players() {
    let env = TestServer::start().await.unwrap();
    let _ = env.issue_player("Bob", &Game::Minecraft).await.unwrap();
    let _ = env.issue_player("Carol", &Game::Minecraft).await.unwrap();

    let resp = env
        .admin_client()
        .unwrap()
        .get(format!("{}{}", env.base_url, ENDPOINT))
        .send()
        .await
        .unwrap();
    HttpAssert::status(resp.status().as_u16(), 200);

    let body: serde_json::Value = resp.json().await.unwrap();
    let names: Vec<String> = body["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["gamertag"].as_str().unwrap().to_string())
        .collect();

    assert!(names.contains(&"Bob".to_string()), "Bob missing: {:?}", names);
    assert!(
        names.contains(&"Carol".to_string()),
        "Carol missing: {:?}",
        names
    );
}

// Nothing in a test server holds a voice connection, so every row is offline. The field
// still has to be present and false, because the pane renders a status from it and a
// missing key would read as an unknown state.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reports_players_as_offline_when_no_voice_connection_exists() {
    let env = TestServer::start().await.unwrap();
    let _ = env.issue_player("Bob", &Game::Minecraft).await.unwrap();

    let resp = env
        .admin_client()
        .unwrap()
        .get(format!("{}{}?search=Bob", env.base_url, ENDPOINT))
        .send()
        .await
        .unwrap();
    let body: serde_json::Value = resp.json().await.unwrap();

    assert_eq!(body["items"][0]["connected"].as_bool().unwrap(), false);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn search_narrows_the_page_and_the_total() {
    let env = TestServer::start().await.unwrap();
    let _ = env.issue_player("Bob", &Game::Minecraft).await.unwrap();
    let _ = env.issue_player("Carol", &Game::Minecraft).await.unwrap();

    let resp = env
        .admin_client()
        .unwrap()
        .get(format!("{}{}?search=Caro", env.base_url, ENDPOINT))
        .send()
        .await
        .unwrap();
    let body: serde_json::Value = resp.json().await.unwrap();

    assert_eq!(body["total"].as_u64().unwrap(), 1);
    assert_eq!(body["items"][0]["gamertag"].as_str().unwrap(), "Carol");
}

// A banned player is the one an operator most needs to find, in order to unban them.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_banished_player_is_listed_with_the_flag_set() {
    let env = TestServer::start().await.unwrap();
    let _ = env.issue_player("Griefer", &Game::Minecraft).await.unwrap();
    env.mark_banished("Griefer", &Game::Minecraft, true)
        .await
        .unwrap();

    let resp = env
        .admin_client()
        .unwrap()
        .get(format!("{}{}?search=Griefer", env.base_url, ENDPOINT))
        .send()
        .await
        .unwrap();
    let body: serde_json::Value = resp.json().await.unwrap();

    assert_eq!(body["items"][0]["banished"].as_bool().unwrap(), true);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn page_size_is_clamped_to_the_maximum() {
    let env = TestServer::start().await.unwrap();
    let resp = env
        .admin_client()
        .unwrap()
        .get(format!("{}{}?page_size=9999", env.base_url, ENDPOINT))
        .send()
        .await
        .unwrap();
    let body: serde_json::Value = resp.json().await.unwrap();

    assert_eq!(body["page_size"].as_u64().unwrap(), 100);
}

// An explicit grant has to appear in the effective set, or the pane cannot badge who
// holds what.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reports_a_granted_permission_in_the_effective_set() {
    let env = TestServer::start().await.unwrap();
    let _ = env
        .issue_player_with_perm("Helper", &Game::Minecraft, "audio_upload")
        .await
        .unwrap();

    let resp = env
        .admin_client()
        .unwrap()
        .get(format!("{}{}?search=Helper", env.base_url, ENDPOINT))
        .send()
        .await
        .unwrap();
    let body: serde_json::Value = resp.json().await.unwrap();

    let permissions: Vec<String> = body["items"][0]["permissions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap().to_string())
        .collect();
    assert!(
        permissions.contains(&"audio_upload".to_string()),
        "granted permission missing: {:?}",
        permissions
    );
}
