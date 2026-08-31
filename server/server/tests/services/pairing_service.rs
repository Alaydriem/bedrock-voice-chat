use std::time::Duration;

use bvc_server_lib::services::PairingService;
use bvc_server_lib::services::pairing_service::RedeemOutcome;
use common::structs::relay::Capability;
use sea_orm::EntityTrait;

use crate::harness::DatabaseFixture;

const NODE: &str = "aaaabbbbccccddddeeeeffff0000111122223333444455556666777788889999";
const OTHER_NODE: &str = "9999888877776666555544443333222211110000ffffeeeeddddccccbbbbaaaa";
const WRONG: &str = "ZZZZZZZZ";

#[tokio::test]
async fn a_code_is_redeemable_exactly_once() {
    let db = DatabaseFixture::create().await.expect("fixture");
    let code = PairingService::mint(&db.connection, "svc-bridge", PairingService::DEFAULT_TTL)
        .await
        .expect("mint");

    let first = PairingService::redeem(&db.connection, NODE, &code, &["W1".to_string()])
        .await
        .expect("redeem");
    let second = PairingService::redeem(&db.connection, OTHER_NODE, &code, &["W1".to_string()])
        .await
        .expect("redeem");

    assert!(matches!(first, RedeemOutcome::Paired { .. }));
    assert!(matches!(second, RedeemOutcome::Spent));
}

#[tokio::test]
async fn an_expired_code_is_refused() {
    let db = DatabaseFixture::create().await.expect("fixture");
    let code = PairingService::mint(&db.connection, "svc-bridge", Duration::from_secs(0))
        .await
        .expect("mint");

    let outcome = PairingService::redeem(&db.connection, NODE, &code, &["W1".to_string()])
        .await
        .expect("redeem");

    assert!(matches!(outcome, RedeemOutcome::Expired));
}

#[tokio::test]
async fn an_unknown_code_is_refused() {
    let db = DatabaseFixture::create().await.expect("fixture");
    PairingService::mint(&db.connection, "svc-bridge", PairingService::DEFAULT_TTL)
        .await
        .expect("mint");

    let outcome = PairingService::redeem(&db.connection, NODE, WRONG, &["W1".to_string()])
        .await
        .expect("redeem");

    assert!(matches!(outcome, RedeemOutcome::Unknown));
}

// The attempt budget is the half tethera left open: a code with no counter is a guessing
// surface bounded only by its window.
#[tokio::test]
async fn repeated_wrong_attempts_spend_the_window() {
    let db = DatabaseFixture::create().await.expect("fixture");
    let code = PairingService::mint(&db.connection, "svc-bridge", PairingService::DEFAULT_TTL)
        .await
        .expect("mint");

    for _ in 0..PairingService::MAX_ATTEMPTS {
        PairingService::redeem(&db.connection, NODE, WRONG, &["W1".to_string()])
            .await
            .expect("redeem");
    }

    let outcome = PairingService::redeem(&db.connection, NODE, &code, &["W1".to_string()])
        .await
        .expect("redeem");

    assert!(matches!(outcome, RedeemOutcome::Spent));
}

#[tokio::test]
async fn an_already_granted_node_does_not_spend_a_second_code() {
    let db = DatabaseFixture::create().await.expect("fixture");
    let first = PairingService::mint(&db.connection, "svc-bridge", PairingService::DEFAULT_TTL)
        .await
        .expect("mint");
    let second = PairingService::mint(&db.connection, "other", PairingService::DEFAULT_TTL)
        .await
        .expect("mint");

    PairingService::redeem(&db.connection, NODE, &first, &["W1".to_string()])
        .await
        .expect("redeem");
    let outcome = PairingService::redeem(&db.connection, NODE, &second, &["W1".to_string()])
        .await
        .expect("redeem");

    assert!(matches!(outcome, RedeemOutcome::AlreadyPaired { .. }));

    let still_usable =
        PairingService::redeem(&db.connection, OTHER_NODE, &second, &["W1".to_string()])
            .await
            .expect("redeem");
    assert!(matches!(still_usable, RedeemOutcome::Paired { .. }));
}

#[tokio::test]
async fn a_paired_grant_carries_the_label_its_code_was_minted_under() {
    let db = DatabaseFixture::create().await.expect("fixture");
    let code = PairingService::mint(&db.connection, "my-bridge", PairingService::DEFAULT_TTL)
        .await
        .expect("mint");

    let outcome = PairingService::redeem(&db.connection, NODE, &code, &["W1".to_string()])
        .await
        .expect("redeem");

    match outcome {
        RedeemOutcome::Paired { label, .. } => assert_eq!(label, "my-bridge"),
        other => panic!("expected Paired, got {other:?}"),
    }
}

// Pairing must never widen what a bridge reaches. Anything beyond carrying voice stays
// reachable only through a `peers` block an operator wrote.
#[tokio::test]
async fn a_paired_grant_carries_only_carry_speakers() {
    let db = DatabaseFixture::create().await.expect("fixture");
    let code = PairingService::mint(&db.connection, "svc-bridge", PairingService::DEFAULT_TTL)
        .await
        .expect("mint");

    let outcome = PairingService::redeem(&db.connection, NODE, &code, &["W1".to_string()])
        .await
        .expect("redeem");

    match outcome {
        RedeemOutcome::Paired { capabilities, .. } => {
            assert_eq!(capabilities, vec![Capability::CarrySpeakers]);
        }
        other => panic!("expected Paired, got {other:?}"),
    }
}

#[tokio::test]
async fn revoking_a_label_removes_its_grant() {
    let db = DatabaseFixture::create().await.expect("fixture");
    let code = PairingService::mint(&db.connection, "svc-bridge", PairingService::DEFAULT_TTL)
        .await
        .expect("mint");
    PairingService::redeem(&db.connection, NODE, &code, &["W1".to_string()])
        .await
        .expect("redeem");

    let removed = PairingService::revoke(&db.connection, "svc-bridge")
        .await
        .expect("revoke");

    assert_eq!(removed, 1);
    assert!(
        PairingService::paired(&db.connection)
            .await
            .expect("paired")
            .is_empty()
    );
}

// The whole point of a digest column: the value an operator typed must not be recoverable
// from the database.
#[tokio::test]
async fn the_plaintext_is_not_stored() {
    let db = DatabaseFixture::create().await.expect("fixture");
    let code = PairingService::mint(&db.connection, "svc-bridge", PairingService::DEFAULT_TTL)
        .await
        .expect("mint");

    let rows = entity::peer_pairing_code::Entity::find()
        .all(&db.connection)
        .await
        .expect("rows");

    assert_eq!(rows.len(), 1);
    assert!(!rows[0].code_digest.contains(&code));
}
