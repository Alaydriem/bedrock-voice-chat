use bvc_server_lib::services::{AccessTokenService, TokenFormat};

use crate::harness::DatabaseFixture;

#[tokio::test]
async fn a_minted_token_is_listed_and_its_secret_is_not_stored() {
    let db = DatabaseFixture::create().await.expect("fixture");

    let minted = AccessTokenService::mint_in(&db.connection)
        .await
        .expect("mint");

    let rows = AccessTokenService::list_in(&db.connection)
        .await
        .expect("list");

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, minted.id);
    assert!(rows[0].revoked_at.is_none());
    assert!(minted.revoked.is_none());

    let loaded = AccessTokenService::load_in(&db.connection)
        .await
        .expect("load");
    let cached = loaded.get(&minted.id).expect("cached");
    let (_, secret) = TokenFormat::parse(&minted.token).expect("parses");

    assert_ne!(cached.secret_hash, secret);
    assert_eq!(cached.secret_hash, TokenFormat::hash(secret));
}

// Revocation is a timestamp, not a delete: the row has to stay so an operator can see what
// was retired, and so the id is never reissued to a different credential.
#[tokio::test]
async fn revoking_marks_the_row_and_keeps_it() {
    let db = DatabaseFixture::create().await.expect("fixture");
    let minted = AccessTokenService::mint_in(&db.connection)
        .await
        .expect("mint");

    let revoked = AccessTokenService::revoke_in(&db.connection, &minted.id)
        .await
        .expect("revoke");

    assert!(revoked);

    let rows = AccessTokenService::list_in(&db.connection)
        .await
        .expect("list");

    assert_eq!(rows.len(), 1);
    assert!(rows[0].revoked_at.is_some());
}

#[tokio::test]
async fn revoking_an_unknown_id_reports_that_it_did_nothing() {
    let db = DatabaseFixture::create().await.expect("fixture");

    let revoked = AccessTokenService::revoke_in(&db.connection, "AbCdEfGh")
        .await
        .expect("revoke");

    assert!(!revoked);
}

#[tokio::test]
async fn two_mints_produce_distinct_ids() {
    let db = DatabaseFixture::create().await.expect("fixture");

    let first = AccessTokenService::mint_in(&db.connection)
        .await
        .expect("first");
    let second = AccessTokenService::mint_in(&db.connection)
        .await
        .expect("second");

    assert_ne!(first.id, second.id);
    assert_ne!(first.token, second.token);
}

#[tokio::test]
async fn rotating_issues_a_new_id_and_retires_the_old_one() {
    let db = DatabaseFixture::create().await.expect("fixture");
    let original = AccessTokenService::mint_in(&db.connection)
        .await
        .expect("mint");

    let rotated = AccessTokenService::rotate_in(&db.connection, &original.id)
        .await
        .expect("rotate");

    assert_ne!(rotated.id, original.id);
    assert_eq!(rotated.revoked.as_deref(), Some(original.id.as_str()));

    let rows = AccessTokenService::list_in(&db.connection)
        .await
        .expect("list");
    let old = rows.iter().find(|row| row.id == original.id).expect("old");
    let new = rows.iter().find(|row| row.id == rotated.id).expect("new");

    assert!(old.revoked_at.is_some());
    assert!(new.revoked_at.is_none());
}

// A rotation that mints without retiring leaves two live credentials and an operator who
// believes there is one. Nothing may be written when the target does not exist.
#[tokio::test]
async fn rotating_an_unknown_id_writes_nothing() {
    let db = DatabaseFixture::create().await.expect("fixture");

    let result = AccessTokenService::rotate_in(&db.connection, "AbCdEfGh").await;

    assert!(result.is_err());
    assert!(
        AccessTokenService::list_in(&db.connection)
            .await
            .expect("list")
            .is_empty()
    );
}
