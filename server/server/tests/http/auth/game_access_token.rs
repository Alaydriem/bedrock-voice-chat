//! `GameAccessToken`
//!
//! Contract:
//! - the token is carried as `Authorization: Bearer <token>`
//! - `X-MC-Access-Token` is refused: this is a forced upgrade, not a deprecation
//! - the `Bearer ` prefix is required, so an ncryptf `Authorization` value is not a credential
//! - a wrong token, or no token at all, is refused
//!
//! Not a timing fix: the comparison was already constant-time and still is. Only the header
//! changed.

use crate::harness::{HttpAssert, TestServer};

// Mounted by the harness and behind the guard. The query values do not have to resolve to a
// real player: the guard is what is under test, and it runs before the handler.
const ENDPOINT: &str = "/api/state?id=Steve&game=minecraft";
const TOKEN: &str = "test-mc-token";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn accepts_the_token_as_a_bearer_credential() {
    let env = TestServer::start().await.unwrap();

    let response = env
        .noauth_client()
        .unwrap()
        .get(format!("{}{}", env.base_url, ENDPOINT))
        .header("Authorization", format!("Bearer {TOKEN}"))
        .send()
        .await
        .unwrap();

    HttpAssert::status(response.status().as_u16(), 200);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn refuses_a_wrong_bearer_token() {
    let env = TestServer::start().await.unwrap();

    let response = env
        .noauth_client()
        .unwrap()
        .get(format!("{}{}", env.base_url, ENDPOINT))
        .header("Authorization", "Bearer not-the-token")
        .send()
        .await
        .unwrap();

    HttpAssert::status(response.status().as_u16(), 403);
}

// A forced upgrade, not a deprecation. A server on this version requires a mod on this
// version; the legacy header carrying the correct token is still refused.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn refuses_the_legacy_header_even_with_the_correct_token() {
    let env = TestServer::start().await.unwrap();

    let response = env
        .noauth_client()
        .unwrap()
        .get(format!("{}{}", env.base_url, ENDPOINT))
        .header("X-MC-Access-Token", TOKEN)
        .send()
        .await
        .unwrap();

    assert_ne!(
        response.status().as_u16(),
        200,
        "X-MC-Access-Token is no longer a credential"
    );
}

// ncryptf also uses `Authorization`, on `/api/auth/*`. No route carries both guards today,
// but a parser that took any Authorization value would accept an HMAC parameter string as a
// bearer credential the moment one did.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn does_not_accept_an_ncryptf_authorization_value() {
    let env = TestServer::start().await.unwrap();

    let response = env
        .noauth_client()
        .unwrap()
        .get(format!("{}{}", env.base_url, ENDPOINT))
        .header("Authorization", format!("HMAC {TOKEN},somesignature,somesalt"))
        .send()
        .await
        .unwrap();

    assert_ne!(
        response.status().as_u16(),
        200,
        "an ncryptf Authorization value must not be read as a bearer token"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn refuses_a_request_carrying_neither_header() {
    let env = TestServer::start().await.unwrap();

    let response = env
        .noauth_client()
        .unwrap()
        .get(format!("{}{}", env.base_url, ENDPOINT))
        .send()
        .await
        .unwrap();

    assert_ne!(response.status().as_u16(), 200);
}
