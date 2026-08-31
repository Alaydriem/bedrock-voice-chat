use std::sync::Arc;

use bvc_relay_service::db::Db;
use bvc_relay_service::registry::ClaimService;

async fn claims() -> Arc<ClaimService> {
    let conn = Arc::new(Db::connect("sqlite::memory:").await.expect("connects"));
    ClaimService::new_shared(conn)
}

// The claim is what carries an enrollment token across an origin boundary. It must
// hand it over exactly once: a replayable claim is a token anyone who sees the URL
// can take.
#[tokio::test]
async fn a_claim_yields_its_token_exactly_once() {
    let claims = claims().await;
    let id = claims.store("bvcenroll-abc").await.expect("stores");

    assert_eq!(
        claims.redeem(&id).await.expect("redeems"),
        Some("bvcenroll-abc".to_string())
    );
    assert_eq!(claims.redeem(&id).await.expect("second redeem"), None);
}

// An unknown id is absent rather than an error. The page redeems whatever the redirect
// handed it, and a stale bookmark must read as "nothing here" rather than a failure.
#[tokio::test]
async fn an_unknown_claim_is_absent() {
    let claims = claims().await;

    assert_eq!(claims.redeem("nonexistent").await.expect("redeems"), None);
}

// Claim ids are unguessable. The id travels in a redirect URL, so anything derived
// from the token or issued in sequence would let one member take another's.
#[tokio::test]
async fn claim_ids_are_distinct_and_do_not_contain_the_token() {
    let claims = claims().await;

    let first = claims.store("bvcenroll-abc").await.expect("stores");
    let second = claims.store("bvcenroll-abc").await.expect("stores");

    assert_ne!(first, second);
    assert!(!first.contains("bvcenroll-abc"));
}

// Expiry is enforced on read, not by a sweeper. A claim the page never redeemed must
// stop being redeemable whether or not anything has run since.
#[tokio::test]
async fn an_expired_claim_is_absent() {
    let claims = claims().await;
    let id = claims
        .store_expiring_at("bvcenroll-abc", 0)
        .await
        .expect("stores an already-expired claim");

    assert_eq!(claims.redeem(&id).await.expect("redeems"), None);
}
