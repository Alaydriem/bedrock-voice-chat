use bvc_relay_service::entity::play_name;
use bvc_relay_service::migration::Migrator;
use sea_orm::{ConnectOptions, ConnectionTrait, EntityTrait};
use sea_orm_migration::MigratorTrait;

// A registry upgraded in place keeps every published name, with no port until the
// node's next check-in proves one.
#[tokio::test]
async fn the_port_column_is_added_to_existing_play_names() {
    let mut options = ConnectOptions::new("sqlite::memory:".to_string());
    options.max_connections(1).sqlx_logging(false);
    let conn = sea_orm::Database::connect(options).await.expect("connects");

    Migrator::up(&conn, Some(5))
        .await
        .expect("migrations up to the play name step run");
    conn.execute_unprepared(
        "INSERT INTO play_name (node_id, name, claimed_from, published_address, created_at) \
         VALUES ('node-a', 'brave-otter-mesa', '203.0.113.7', '198.51.100.4', 0)",
    )
    .await
    .expect("a published row inserts under the old schema");

    Migrator::up(&conn, None)
        .await
        .expect("the port migration runs");

    let row = play_name::Entity::find()
        .one(&conn)
        .await
        .expect("the row loads")
        .expect("the row survives");
    assert_eq!(row.published_address.as_deref(), Some("198.51.100.4"));
    assert_eq!(row.published_port, None);
}
