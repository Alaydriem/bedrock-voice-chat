use bvc_server_lib::runtime::access_token::AccessTokenManager;
use bvc_server_lib::runtime::{SecretName, SecretStore};
use tempfile::TempDir;

use crate::harness::DatabaseFixture;

#[tokio::test]
async fn a_configured_token_wins_and_is_mirrored_into_the_database() {
    let db = DatabaseFixture::create().await.expect("fixture");
    let dir = TempDir::new().expect("tempdir");
    let manager = AccessTokenManager::new(dir.path().to_str().unwrap());

    let token = manager
        .resolve(&db.connection, "configured-token")
        .await
        .expect("resolve");

    assert_eq!(token, "configured-token");
    assert_eq!(
        SecretStore::read(&db.connection, SecretName::MinecraftAccessToken)
            .await
            .expect("read"),
        Some("configured-token".to_string())
    );
}

// The upgrade path: beta.20 wrote the token to `<certs_path>/access_token`.
#[tokio::test]
async fn an_existing_token_file_is_imported() {
    let db = DatabaseFixture::create().await.expect("fixture");
    let dir = TempDir::new().expect("tempdir");
    std::fs::write(dir.path().join("access_token"), "beta-20-token").expect("write");
    let manager = AccessTokenManager::new(dir.path().to_str().unwrap());

    let token = manager.resolve(&db.connection, "").await.expect("resolve");

    assert_eq!(
        token, "beta-20-token",
        "a mod configured against this token must keep working across the upgrade"
    );
}

// A fresh install must not leave the token on disk. That file is what makes a container
// need a persistent volume.
#[tokio::test]
async fn a_generated_token_is_never_written_to_disk() {
    let db = DatabaseFixture::create().await.expect("fixture");
    let dir = TempDir::new().expect("tempdir");
    let manager = AccessTokenManager::new(dir.path().to_str().unwrap());

    let token = manager.resolve(&db.connection, "").await.expect("resolve");

    assert_eq!(token.len(), 32);
    assert!(
        !dir.path().join("access_token").exists(),
        "the database is the only durable copy"
    );
}

#[tokio::test]
async fn a_second_boot_reuses_the_stored_token() {
    let db = DatabaseFixture::create().await.expect("fixture");
    let dir = TempDir::new().expect("tempdir");
    let manager = AccessTokenManager::new(dir.path().to_str().unwrap());

    let first = manager.resolve(&db.connection, "").await.expect("first");
    let second = manager.resolve(&db.connection, "").await.expect("second");

    assert_eq!(first, second);
}
