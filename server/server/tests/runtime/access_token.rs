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

    assert_eq!(token.as_deref(), Some("configured-token"));
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
        token.as_deref(),
        Some("beta-20-token"),
        "a mod configured against this token must keep working across the upgrade"
    );
}

// A deployment that configures nothing gets no scalar credential at all. Generating one
// produced a secret no operator could read, which is what identified tokens replaced.
#[tokio::test]
async fn nothing_is_generated_when_a_deployment_configures_no_token() {
    let db = DatabaseFixture::create().await.expect("fixture");
    let dir = TempDir::new().expect("tempdir");
    let manager = AccessTokenManager::new(dir.path().to_str().unwrap());

    let token = manager.resolve(&db.connection, "").await.expect("resolve");

    assert!(token.is_none());
    assert!(
        SecretStore::read(&db.connection, SecretName::MinecraftAccessToken)
            .await
            .expect("read")
            .is_none(),
        "nothing may be written for a deployment that asked for nothing"
    );
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

    let first = manager
        .resolve(&db.connection, "configured-token")
        .await
        .expect("first");
    let second = manager.resolve(&db.connection, "").await.expect("second");

    assert_eq!(first, second);
}
