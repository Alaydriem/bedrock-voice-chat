use std::sync::Arc;

use bvc_relay_service::db::Db;
use bvc_relay_service::storage::NodeKeyStore;
use sea_orm::DatabaseConnection;

async fn conn() -> Arc<DatabaseConnection> {
    Arc::new(Db::connect("sqlite::memory:").await.expect("connects"))
}

// The key IS the registry's identity: every enrolled server holds a peer link naming
// it. A second start that generated a fresh one would make all of them dial a node that
// no longer answers, and nothing would report it until one tried.
#[tokio::test]
async fn the_key_survives_a_restart() {
    let conn = conn().await;

    let first = NodeKeyStore::new(conn.clone())
        .resolve()
        .await
        .expect("generates");
    let second = NodeKeyStore::new(conn).resolve().await.expect("reads back");

    assert_eq!(first, second);
}

// Two registries are two identities. A generator that returned the same key twice would
// be the worst kind of bug here — silent, and only visible as one registry answering
// for another.
#[tokio::test]
async fn separate_databases_hold_separate_identities() {
    let first = NodeKeyStore::new(conn().await)
        .resolve()
        .await
        .expect("generates");
    let second = NodeKeyStore::new(conn().await)
        .resolve()
        .await
        .expect("generates");

    assert_ne!(first, second);
    assert_ne!(first, [0u8; 32]);
}
