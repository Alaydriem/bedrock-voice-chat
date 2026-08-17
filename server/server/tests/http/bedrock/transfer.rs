//! `POST /api/bedrock/transfer`
//!
//! Contract:
//! - the target is stored under the caller's own gamertag, taken from their certificate
//! - the request carries no subject, so a caller cannot write a target for anybody else
//! - Bedrock-only: a certificate for another game is refused

use common::Game;
use common::request::bedrock::TransferTargetRequest;

use crate::harness::http_client::MtlsClient;
use crate::harness::{HttpAssert, TestServer};

const ENDPOINT: &str = "/api/bedrock/transfer";

// The route used to discard the resolved player and key on a caller-supplied xuid, so any
// authenticated player could point another player's BVC Connect handoff anywhere.
//
// There is deliberately no test for "a caller cannot write a target for someone else": the
// request no longer carries a subject at all, so the case is unrepresentable rather than
// rejected, which is the stronger outcome.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_transfer_target_is_stored_under_the_callers_own_gamertag() {
    let env = TestServer::start().await.unwrap();
    let (cert, key) = env
        .issue_player("Traveller", &Game::Minecraft)
        .await
        .unwrap();

    let response = MtlsClient::with_identity(&env.ca_pem, &cert, &key)
        .unwrap()
        .post(format!("{}{}", env.base_url, ENDPOINT))
        .json(&TransferTargetRequest {
            host: "play.example.com".to_string(),
            port: 19132,
        })
        .send()
        .await
        .unwrap();

    HttpAssert::status(response.status().as_u16(), 200);

    let target = env
        .transfer_cache
        .get("Traveller")
        .await
        .expect("a target stored under the caller's own gamertag");
    assert_eq!(target.host, "play.example.com");
    assert_eq!(target.port, 19132);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_request_without_a_client_certificate_is_refused() {
    let env = TestServer::start().await.unwrap();

    let response = env
        .noauth_client()
        .unwrap()
        .post(format!("{}{}", env.base_url, ENDPOINT))
        .json(&TransferTargetRequest {
            host: "play.example.com".to_string(),
            port: 19132,
        })
        .send()
        .await
        .unwrap();

    let status = response.status().as_u16();
    assert!(
        status == 401 || status == 403,
        "expected 401 or 403 without a client certificate, got {status}"
    );
}
