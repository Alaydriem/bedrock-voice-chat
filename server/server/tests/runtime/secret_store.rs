use bvc_server_lib::runtime::{SecretName, SecretStore};
use tempfile::TempDir;

use crate::harness::DatabaseFixture;

#[tokio::test]
async fn a_generated_secret_is_stored_and_reused() {
    let db = DatabaseFixture::create().await.expect("fixture");

    let first = SecretStore::resolve(
        &db.connection,
        SecretName::MinecraftAccessToken,
        None,
        None,
        || "generated".to_string(),
    )
    .await
    .expect("first");

    let second = SecretStore::resolve(
        &db.connection,
        SecretName::MinecraftAccessToken,
        None,
        None,
        || panic!("a stored secret must never be regenerated"),
    )
    .await
    .expect("second");

    assert_eq!(first, "generated");
    assert_eq!(second, "generated");
}

// The upgrade case. A beta.20 deployment has the value in a file and nothing in the
// database; the file's value must become the row rather than being replaced.
#[tokio::test]
async fn a_legacy_file_is_imported_rather_than_replaced() {
    let db = DatabaseFixture::create().await.expect("fixture");
    let dir = TempDir::new().expect("tempdir");
    let legacy = dir.path().join("access_token");
    std::fs::write(&legacy, "from-beta-20").expect("write legacy");

    let resolved = SecretStore::resolve(
        &db.connection,
        SecretName::MinecraftAccessToken,
        None,
        Some(legacy.as_path()),
        || panic!("a legacy file must be imported, never replaced"),
    )
    .await
    .expect("resolve");

    assert_eq!(resolved, "from-beta-20");
    assert_eq!(
        SecretStore::read(&db.connection, SecretName::MinecraftAccessToken)
            .await
            .expect("read"),
        Some("from-beta-20".to_string()),
        "the imported value must be persisted, or the next boot loses it"
    );
}

#[tokio::test]
async fn a_configured_value_overwrites_the_stored_row() {
    let db = DatabaseFixture::create().await.expect("fixture");

    SecretStore::resolve(
        &db.connection,
        SecretName::MinecraftAccessToken,
        None,
        None,
        || "generated".to_string(),
    )
    .await
    .expect("seed");

    let resolved = SecretStore::resolve(
        &db.connection,
        SecretName::MinecraftAccessToken,
        Some("from-config"),
        None,
        || panic!("configured values are authoritative"),
    )
    .await
    .expect("resolve");

    assert_eq!(resolved, "from-config");
    assert_eq!(
        SecretStore::read(&db.connection, SecretName::MinecraftAccessToken)
            .await
            .expect("read"),
        Some("from-config".to_string()),
        "config is mirrored into the database so every later read agrees with it"
    );
}

// A blank or whitespace-only configured value is an unset value, not an instruction to
// store an empty secret. A compose file with `BVC_ACCESS_TOKEN=` must not blank the row.
#[tokio::test]
async fn a_blank_configured_value_is_treated_as_unset() {
    let db = DatabaseFixture::create().await.expect("fixture");

    SecretStore::resolve(
        &db.connection,
        SecretName::MinecraftAccessToken,
        None,
        None,
        || "generated".to_string(),
    )
    .await
    .expect("seed");

    let resolved = SecretStore::resolve(
        &db.connection,
        SecretName::MinecraftAccessToken,
        Some("   "),
        None,
        || panic!("a stored secret must never be regenerated"),
    )
    .await
    .expect("resolve");

    assert_eq!(resolved, "generated");
}

// Two secrets share the table and must not collide.
#[tokio::test]
async fn distinct_names_hold_distinct_values() {
    let db = DatabaseFixture::create().await.expect("fixture");

    SecretStore::resolve(
        &db.connection,
        SecretName::MinecraftAccessToken,
        Some("token"),
        None,
        || unreachable!(),
    )
    .await
    .expect("token");
    SecretStore::resolve(
        &db.connection,
        SecretName::RelayNodeKey,
        Some("nodekey"),
        None,
        || unreachable!(),
    )
    .await
    .expect("node key");

    assert_eq!(
        SecretStore::read(&db.connection, SecretName::MinecraftAccessToken)
            .await
            .expect("read token"),
        Some("token".to_string())
    );
    assert_eq!(
        SecretStore::read(&db.connection, SecretName::RelayNodeKey)
            .await
            .expect("read node key"),
        Some("nodekey".to_string())
    );
}
