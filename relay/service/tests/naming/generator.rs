use bvc_relay_service::db::Db;
use bvc_relay_service::entity::{RegistrationState, registration, retired_name};
use bvc_relay_service::naming::{NameGenerator, WordList};
use sea_orm::{ActiveModelTrait, ActiveValue, EntityTrait};

async fn hold(conn: &sea_orm::DatabaseConnection, node_id: &str, name: &str, member: &str) {
    registration::ActiveModel {
        node_id: ActiveValue::Set(node_id.to_string()),
        name: ActiveValue::Set(name.to_string()),
        discord_user_id: ActiveValue::Set(member.to_string()),
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
    .insert(conn)
    .await
    .expect("the registration inserts");
}

// A retired name is never offered again. The previous holder's address is still in
// operator configuration and in client history, so reassignment would hand the next
// holder a publicly trusted certificate for a name other people still resolve.
#[tokio::test]
async fn a_retired_name_is_never_available_again() {
    let conn = Db::connect("sqlite::memory:").await.expect("connects");
    let name = "creeper-diorite-badlands";
    assert!(
        NameGenerator::is_available(&conn, name)
            .await
            .expect("queries")
    );

    NameGenerator::retire(&conn, name).await.expect("retires");

    assert!(
        !NameGenerator::is_available(&conn, name)
            .await
            .expect("queries")
    );
}

// A name held by a live registration is not offered either, even though it has not
// been retired.
#[tokio::test]
async fn a_name_a_live_registration_holds_is_not_available() {
    let conn = Db::connect("sqlite::memory:").await.expect("connects");
    let name = "redstone-piglin-taiga";
    hold(&conn, "node-a", name, "member-1").await;

    assert!(
        !NameGenerator::is_available(&conn, name)
            .await
            .expect("queries")
    );
}

// Retiring the same name twice is not an error. Suspension and retirement can both
// reach it, and a duplicate-key failure there would abort an unrelated pass.
#[tokio::test]
async fn retiring_a_name_twice_is_not_an_error() {
    let conn = Db::connect("sqlite::memory:").await.expect("connects");

    NameGenerator::retire(&conn, "creeper-diorite-badlands")
        .await
        .expect("retires");
    NameGenerator::retire(&conn, "creeper-diorite-badlands")
        .await
        .expect("retires again");

    let row = retired_name::Entity::find_by_id("creeper-diorite-badlands")
        .one(&conn)
        .await
        .expect("query succeeds");

    assert!(row.is_some(), "retirement must be recorded");
}

// Assignment produces a name the registry considers available, and a distinct one
// each time it is called against a registry that holds the previous answer.
#[tokio::test]
async fn assignment_produces_an_available_name() {
    let conn = Db::connect("sqlite::memory:").await.expect("connects");
    let generator = NameGenerator::new();

    let first = generator.assign(&conn).await.expect("assigns");
    hold(&conn, "node-a", &first, "member-1").await;

    let second = generator.assign(&conn).await.expect("assigns again");

    assert_ne!(second, first, "a held name must not be offered again");
    assert!(
        NameGenerator::is_available(&conn, &second)
            .await
            .expect("queries")
    );
}

// The word list carries no profanity and no Mojang or Microsoft mark. A name in the
// relay's own zone is the operator's public address; a bad word in it is the
// relay's problem, not theirs.
#[test]
fn every_word_in_the_list_is_clean() {
    for word in WordList::ADJECTIVES
        .iter()
        .chain(WordList::NOUNS)
        .chain(WordList::PLACES)
    {
        assert!(WordList::is_clean(word), "{word} must pass the deny list");
    }
}
