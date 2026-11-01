use bvc_server_lib::runtime::AssignedNameStore;
use bvc_server_lib::services::play::PlayNameStore;

use crate::harness::DatabaseFixture;

// A server that has claimed a play name starts on it without asking the registry again.
#[tokio::test]
async fn a_written_play_name_is_read_back() {
    let db = DatabaseFixture::create().await.expect("fixture");

    PlayNameStore::write(&db.connection, "brave-otter-mesa-0042.play.bedrockvc.stream")
        .await
        .expect("writes");

    assert_eq!(
        PlayNameStore::read(&db.connection).await.expect("reads"),
        Some("brave-otter-mesa-0042.play.bedrockvc.stream".to_string())
    );
}

// No name is the condition that makes the claim run.
#[tokio::test]
async fn a_server_that_never_claimed_has_no_play_name() {
    let db = DatabaseFixture::create().await.expect("fixture");

    assert_eq!(PlayNameStore::read(&db.connection).await.expect("reads"), None);
}

// The play name and the assigned name are different rows. A server that switches between
// the two features must not read one back as the other.
#[tokio::test]
async fn the_play_name_and_the_assigned_name_do_not_share_a_row() {
    let db = DatabaseFixture::create().await.expect("fixture");
    AssignedNameStore::write(&db.connection, "assigned.bedrockvc.stream")
        .await
        .expect("writes the assigned name");

    assert_eq!(PlayNameStore::read(&db.connection).await.expect("reads"), None);

    PlayNameStore::write(&db.connection, "claimed.play.bedrockvc.stream")
        .await
        .expect("writes the play name");

    assert_eq!(
        AssignedNameStore::read(&db.connection).await.expect("reads"),
        Some("assigned.bedrockvc.stream".to_string())
    );
}

// A name the registry no longer binds is forgotten so the next boot claims again.
#[tokio::test]
async fn a_cleared_play_name_reads_as_none() {
    let db = DatabaseFixture::create().await.expect("fixture");
    PlayNameStore::write(&db.connection, "gone.play.bedrockvc.stream")
        .await
        .expect("writes");

    assert!(PlayNameStore::clear(&db.connection).await.expect("clears"));

    assert_eq!(PlayNameStore::read(&db.connection).await.expect("reads"), None);
}
