use bvc_relay_service::db::Db;
use bvc_relay_service::entity::{RegistrationState, registration};
use sea_orm::{ActiveModelTrait, ActiveValue, EntityTrait};

// Migrations run on connect. A caller that has a connection has a schema; there is
// no second call to forget.
#[tokio::test]
async fn connecting_runs_migrations() {
    let conn = Db::connect("sqlite::memory:").await.expect("connects");

    let rows = registration::Entity::find()
        .all(&conn)
        .await
        .expect("the registration table exists");

    assert!(rows.is_empty());
}

// One name per Discord user is the entitlement rule, enforced in the schema rather
// than only in the service. A second row for the same member must fail at the
// database, so no code path can create one by forgetting the check.
#[tokio::test]
async fn a_second_registration_for_one_discord_user_is_refused() {
    let conn = Db::connect("sqlite::memory:").await.expect("connects");

    registration::ActiveModel {
        node_id: ActiveValue::Set("node-a".to_string()),
        name: ActiveValue::Set("creeper-diorite-badlands".to_string()),
        discord_user_id: ActiveValue::Set("member-1".to_string()),
        state: ActiveValue::Set(RegistrationState::Active.as_str().to_string()),
        declared_address: ActiveValue::Set(None),
        address_verified_at: ActiveValue::Set(None),
        entitlement_checked_at: ActiveValue::Set(None),
        entitlement_ok: ActiveValue::Set(true),
        validated_at: ActiveValue::Set(None),
        validation_failures: ActiveValue::Set(0),
        created_at: ActiveValue::Set(0),
        suspended_at: ActiveValue::Set(None),
        retired_at: ActiveValue::Set(None),
    }
    .insert(&conn)
    .await
    .expect("the first registration inserts");

    let second = registration::ActiveModel {
        node_id: ActiveValue::Set("node-b".to_string()),
        name: ActiveValue::Set("redstone-piglin-taiga".to_string()),
        discord_user_id: ActiveValue::Set("member-1".to_string()),
        state: ActiveValue::Set(RegistrationState::Active.as_str().to_string()),
        declared_address: ActiveValue::Set(None),
        address_verified_at: ActiveValue::Set(None),
        entitlement_checked_at: ActiveValue::Set(None),
        entitlement_ok: ActiveValue::Set(true),
        validated_at: ActiveValue::Set(None),
        validation_failures: ActiveValue::Set(0),
        created_at: ActiveValue::Set(0),
        suspended_at: ActiveValue::Set(None),
        retired_at: ActiveValue::Set(None),
    }
    .insert(&conn)
    .await;

    assert!(
        second.is_err(),
        "a second name for one member must be refused"
    );
}

// A name is unique across every registration, live or not. Uniqueness here plus the
// retired_name table is what makes reassignment impossible.
#[tokio::test]
async fn two_registrations_cannot_share_a_name() {
    let conn = Db::connect("sqlite::memory:").await.expect("connects");

    registration::ActiveModel {
        node_id: ActiveValue::Set("node-a".to_string()),
        name: ActiveValue::Set("creeper-diorite-badlands".to_string()),
        discord_user_id: ActiveValue::Set("member-1".to_string()),
        state: ActiveValue::Set(RegistrationState::Active.as_str().to_string()),
        declared_address: ActiveValue::Set(None),
        address_verified_at: ActiveValue::Set(None),
        entitlement_checked_at: ActiveValue::Set(None),
        entitlement_ok: ActiveValue::Set(true),
        validated_at: ActiveValue::Set(None),
        validation_failures: ActiveValue::Set(0),
        created_at: ActiveValue::Set(0),
        suspended_at: ActiveValue::Set(None),
        retired_at: ActiveValue::Set(None),
    }
    .insert(&conn)
    .await
    .expect("the first registration inserts");

    let clash = registration::ActiveModel {
        node_id: ActiveValue::Set("node-b".to_string()),
        name: ActiveValue::Set("creeper-diorite-badlands".to_string()),
        discord_user_id: ActiveValue::Set("member-2".to_string()),
        state: ActiveValue::Set(RegistrationState::Active.as_str().to_string()),
        declared_address: ActiveValue::Set(None),
        address_verified_at: ActiveValue::Set(None),
        entitlement_checked_at: ActiveValue::Set(None),
        entitlement_ok: ActiveValue::Set(true),
        validated_at: ActiveValue::Set(None),
        validation_failures: ActiveValue::Set(0),
        created_at: ActiveValue::Set(0),
        suspended_at: ActiveValue::Set(None),
        retired_at: ActiveValue::Set(None),
    }
    .insert(&conn)
    .await;

    assert!(clash.is_err(), "a name must not be held twice");
}
