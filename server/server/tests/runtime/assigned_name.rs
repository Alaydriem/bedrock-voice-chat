use bvc_server_lib::runtime::AssignedNameStore;

use crate::harness::DatabaseFixture;

// The name survives a relay outage. A server that has enrolled once starts on its
// stored name and its certificate on disk, whether or not the relay answers.
#[tokio::test]
async fn a_written_name_is_read_back() {
    let db = DatabaseFixture::create().await.expect("fixture");

    AssignedNameStore::write(
        &db.connection,
        "creeper-diorite-badlands.bedrockvc.stream",
    )
    .await
    .expect("writes");

    assert_eq!(
        AssignedNameStore::read(&db.connection).await.expect("reads"),
        Some("creeper-diorite-badlands.bedrockvc.stream".to_string())
    );
}

// A server that has never enrolled has no name, and that is not an error — it is the
// condition that makes the enrollment step run.
#[tokio::test]
async fn an_unenrolled_server_has_no_name() {
    let db = DatabaseFixture::create().await.expect("fixture");

    assert_eq!(
        AssignedNameStore::read(&db.connection).await.expect("reads"),
        None
    );
}

// Re-enrolling replaces the stored name rather than appending a second row, so a
// server cannot end up half-remembering two names.
#[tokio::test]
async fn writing_a_second_name_replaces_the_first() {
    let db = DatabaseFixture::create().await.expect("fixture");
    AssignedNameStore::write(&db.connection, "first.bedrockvc.stream")
        .await
        .expect("writes");

    AssignedNameStore::write(&db.connection, "second.bedrockvc.stream")
        .await
        .expect("writes again");

    assert_eq!(
        AssignedNameStore::read(&db.connection).await.expect("reads"),
        Some("second.bedrockvc.stream".to_string())
    );
}

// The name is stored under its own key, so resolving it never collides with the node
// key or the access token sharing the same table.
#[tokio::test]
async fn the_assigned_name_does_not_disturb_the_other_secrets() {
    use bvc_server_lib::runtime::{SecretName, SecretStore};

    let db = DatabaseFixture::create().await.expect("fixture");
    let token = SecretStore::resolve(
        &db.connection,
        SecretName::MinecraftAccessToken,
        None,
        None,
        || "access-token".to_string(),
    )
    .await
    .expect("resolves the access token");

    AssignedNameStore::write(&db.connection, "assigned.bedrockvc.stream")
        .await
        .expect("writes");

    assert_eq!(
        SecretStore::read(&db.connection, SecretName::MinecraftAccessToken)
            .await
            .expect("reads"),
        Some(token)
    );
}
