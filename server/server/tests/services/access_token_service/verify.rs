use std::sync::Arc;

use bvc_server_lib::services::AccessTokenService;

use crate::harness::DatabaseFixture;

#[tokio::test]
async fn a_minted_token_verifies_and_a_wrong_secret_does_not() {
    let db = DatabaseFixture::create().await.expect("fixture");
    let service = AccessTokenService::new_shared(Arc::new(db.connection.clone()), None, false);

    let minted = service.mint().await.expect("mint");

    assert!(service.verify(&minted.token));
    assert!(!service.verify("bvc_AAAAAAAA_AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"));
}

// The invariant that makes removing first-boot generation safe. `Authorization: Bearer `
// with nothing after it presents an empty string, and a constant-time comparison of two
// empty values is true, so an unconfigured server would authenticate every caller.
#[tokio::test]
async fn an_empty_presentation_is_refused_when_no_token_exists() {
    let db = DatabaseFixture::create().await.expect("fixture");
    let service = AccessTokenService::new_shared(Arc::new(db.connection.clone()), None, false);

    assert!(!service.verify(""));
}

#[tokio::test]
async fn an_empty_presentation_is_refused_when_the_scalar_is_blank() {
    let db = DatabaseFixture::create().await.expect("fixture");
    let service =
        AccessTokenService::new_shared(Arc::new(db.connection.clone()), Some(String::new()), false);

    assert!(!service.verify(""));
}

#[tokio::test]
async fn the_legacy_scalar_still_verifies() {
    let db = DatabaseFixture::create().await.expect("fixture");
    let service = AccessTokenService::new_shared(
        Arc::new(db.connection.clone()),
        Some("legacy-value".to_string()),
        true,
    );

    assert!(service.verify("legacy-value"));
    assert!(!service.verify("legacy-valuf"));
}

#[tokio::test]
async fn a_revoked_token_stops_verifying() {
    let db = DatabaseFixture::create().await.expect("fixture");
    let service = AccessTokenService::new_shared(Arc::new(db.connection.clone()), None, false);
    let minted = service.mint().await.expect("mint");

    service.revoke(&minted.id).await.expect("revoke");

    assert!(!service.verify(&minted.token));
}

#[tokio::test]
async fn rotating_swaps_which_token_verifies() {
    let db = DatabaseFixture::create().await.expect("fixture");
    let service = AccessTokenService::new_shared(Arc::new(db.connection.clone()), None, false);
    let original = service.mint().await.expect("mint");

    let rotated = service.rotate(&original.id).await.expect("rotate");

    assert!(!service.verify(&original.token));
    assert!(service.verify(&rotated.token));
}

// The `--local` path: another process writes the row, and this one only learns about it on
// reload. Both directions matter, and the revocation direction is the one a
// refresh-on-mismatch cache would never have caught, because a revoked token still matches
// the cached row.
#[tokio::test]
async fn reload_picks_up_out_of_band_mints_and_revocations() {
    let db = DatabaseFixture::create().await.expect("fixture");
    let service = AccessTokenService::new_shared(Arc::new(db.connection.clone()), None, false);

    let minted = AccessTokenService::mint_in(&db.connection)
        .await
        .expect("mint");

    assert!(!service.verify(&minted.token));
    service.reload().await.expect("reload");
    assert!(service.verify(&minted.token));

    AccessTokenService::revoke_in(&db.connection, &minted.id)
        .await
        .expect("revoke");

    assert!(service.verify(&minted.token));
    service.reload().await.expect("reload");
    assert!(!service.verify(&minted.token));
}

#[tokio::test]
async fn revoking_the_legacy_scalar_is_refused_when_it_is_configured() {
    let db = DatabaseFixture::create().await.expect("fixture");
    let service = AccessTokenService::new_shared(
        Arc::new(db.connection.clone()),
        Some("from-env".to_string()),
        true,
    );

    assert!(service.revoke_legacy().await.is_err());
    assert!(service.verify("from-env"));
}
