use bvc_relay_service::db::Db;
use bvc_relay_service::entity::{play_name, play_zone_state};
use sea_orm::{ConnectionTrait, EntityTrait};

// The zone serial is read by id and advanced by compare-and-set, so the migration
// must leave exactly the one row both rely on.
#[tokio::test]
async fn the_migration_seeds_one_zone_state_row_at_serial_zero() {
    let conn = Db::connect("sqlite::memory:").await.expect("connects");

    let rows = play_zone_state::Entity::find().all(&conn).await.expect("reads");

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, 1);
    assert_eq!(rows[0].serial, 0);
}

// Two nodes drawing the same label must not both keep it: the unique key is what
// settles that race.
#[tokio::test]
async fn two_nodes_cannot_hold_the_same_label() {
    let conn = Db::connect("sqlite::memory:").await.expect("connects");
    let insert = |node: &str| {
        format!(
            "INSERT INTO play_name (node_id, name, claimed_from, created_at) \
             VALUES ('{node}', 'brave-otter-mesa', '203.0.113.7', 0)"
        )
    };

    conn.execute_unprepared(&insert("node-a"))
        .await
        .expect("the first claim inserts");

    assert!(conn.execute_unprepared(&insert("node-b")).await.is_err());
    assert_eq!(
        play_name::Entity::find().all(&conn).await.expect("reads").len(),
        1
    );
}
