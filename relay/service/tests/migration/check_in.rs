use bvc_relay_service::entity::registration;
use bvc_relay_service::migration::Migrator;
use sea_orm::{ConnectOptions, ConnectionTrait, EntityTrait};
use sea_orm_migration::MigratorTrait;

#[tokio::test]
async fn existing_suspended_rows_become_revoked() {
    let mut options = ConnectOptions::new("sqlite::memory:".to_string());
    options.max_connections(1).sqlx_logging(false);
    let conn = sea_orm::Database::connect(options).await.expect("connects");

    Migrator::up(&conn, Some(3))
        .await
        .expect("migrations up to the storage step run");

    conn.execute_unprepared(
        "INSERT INTO registration (node_id, name, discord_user_id, state, entitlement_ok, \
         validation_failures, created_at, suspended_at) \
         VALUES ('node-a', 'creeper-diorite-badlands', 'member-1', 'suspended', 1, 0, 0, 100)",
    )
    .await
    .expect("a suspended row inserts under the old schema");

    Migrator::up(&conn, None)
        .await
        .expect("the check-in migration runs");

    let row = registration::Entity::find()
        .one(&conn)
        .await
        .expect("reads")
        .expect("the row survives");

    assert_eq!(row.state, "revoked");
    assert_eq!(row.revoked_at, Some(100));
    assert_eq!(row.suspended_at, None);
}
