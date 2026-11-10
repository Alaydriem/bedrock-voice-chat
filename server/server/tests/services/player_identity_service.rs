//! Canonicalizing a game server's roster selects the right player.
//!
//! A Floodgate player is named by their gamertag, which removes the separation Floodgate's
//! prefix gave them from Java accounts. Every collision below must leave each name with the
//! person who owns it, never quietly with someone else.

use bvc_server_lib::services::AliasClaim;
use common::traits::player_data::PlayerData;
use common::Game;
use entity::{player, player_identity};
use sea_orm::{ColumnTrait, EntityTrait, ModelTrait, QueryFilter};

use crate::harness::{GameServerReport, GateRows};

const FLOODGATE_UUID: &str = GateRows::FLOODGATE_UUID;
const FLOODGATE_UUID_2: &str = GateRows::FLOODGATE_UUID_2;
const JAVA_UUID: &str = GateRows::JAVA_UUID;
const JAVA_UUID_2: &str = GateRows::JAVA_UUID_2;

#[tokio::test]
async fn a_player_with_an_alternative_identity_is_renamed_to_it() {
    let report = GameServerReport::new().await;
    let mut roster = vec![GameServerReport::player(
        ".Player_One",
        Some("Player One"),
        Some(FLOODGATE_UUID),
    )];

    let claims = report
        .identity
        .canonicalize_roster(&mut roster, &Game::Minecraft)
        .await;

    assert_eq!(roster[0].get_name(), "Player One");
    assert_eq!(roster[0].get_alternative_identity(), None);
    assert_eq!(
        claims,
        vec![AliasClaim::new(".Player_One", "Player One", Some(FLOODGATE_UUID))]
    );
}

#[tokio::test]
async fn a_player_without_an_alternative_identity_is_untouched() {
    let report = GameServerReport::new().await;
    let mut roster = vec![GameServerReport::player("Steve", None, Some(JAVA_UUID))];

    let claims = report
        .identity
        .canonicalize_roster(&mut roster, &Game::Minecraft)
        .await;

    assert_eq!(roster[0].get_name(), "Steve");
    assert!(claims.is_empty());
}

// The gamertag sent as its own alternative is Floodgate's word that the name is that
// player's. The claim on its own name is what later refuses the name to anyone else.
#[tokio::test]
async fn an_alternative_identity_equal_to_the_name_claims_the_name_for_itself() {
    let report = GameServerReport::new().await;
    let mut roster = vec![GameServerReport::player("XboxTag", Some("XboxTag"), Some(JAVA_UUID))];

    let claims = report
        .identity
        .canonicalize_roster(&mut roster, &Game::Minecraft)
        .await;

    assert_eq!(roster[0].get_name(), "XboxTag");
    assert_eq!(
        claims,
        vec![AliasClaim::new("XboxTag", "XboxTag", Some(JAVA_UUID))]
    );
}

// A linked player proved the gamertag "Builder" is theirs. A Java account of the same name
// plays on the server, so its platform UUID is on that record too, and then links to
// another gamertag. The name stays with the player who proved it.
#[tokio::test]
async fn a_java_account_cannot_alias_a_gamertag_its_owner_proved() {
    let report = GameServerReport::new().await;
    report
        .ingest(vec![GameServerReport::player("Builder", Some("Builder"), Some(JAVA_UUID))])
        .await;
    report
        .ingest(vec![GameServerReport::player("Builder", None, Some(JAVA_UUID_2))])
        .await;

    report
        .ingest(vec![GameServerReport::player(
            "Builder",
            Some("OtherTag"),
            Some(JAVA_UUID_2),
        )])
        .await;

    assert_eq!(report.resolve("Builder").await, "Builder");
    let actor = report
        .identity
        .canonical_name("Builder", Some(JAVA_UUID), &Game::Minecraft)
        .await;
    assert_eq!(actor, "Builder");
}

// The same three players, with the Java account on the server before the owner proves the
// gamertag.
#[tokio::test]
async fn a_java_account_seen_first_cannot_alias_a_gamertag_its_owner_proved_later() {
    let report = GameServerReport::new().await;
    report
        .ingest(vec![GameServerReport::player("Builder", None, Some(JAVA_UUID_2))])
        .await;
    report
        .ingest(vec![GameServerReport::player("Builder", Some("Builder"), Some(JAVA_UUID))])
        .await;

    report
        .ingest(vec![GameServerReport::player(
            "Builder",
            Some("OtherTag"),
            Some(JAVA_UUID_2),
        )])
        .await;

    assert_eq!(report.resolve("Builder").await, "Builder");
}

#[tokio::test]
async fn an_empty_alternative_identity_makes_no_claim() {
    let report = GameServerReport::new().await;
    let mut roster = vec![GameServerReport::player("Steve", Some(""), Some(JAVA_UUID))];

    let claims = report
        .identity
        .canonicalize_roster(&mut roster, &Game::Minecraft)
        .await;

    assert_eq!(roster[0].get_name(), "Steve");
    assert!(claims.is_empty());
}

// A Java account named Steve and a Floodgate player whose gamertag is Steve both arrive
// as "Steve". The gamertag belongs to the Xbox account; the Java player could never sign in
// as it.
#[tokio::test]
async fn a_roster_with_two_players_named_alike_keeps_the_xbox_verified_one() {
    let report = GameServerReport::new().await;

    let roster = report
        .ingest(vec![
            GameServerReport::player("Steve", None, Some(JAVA_UUID_2)),
            GameServerReport::player("Steve", None, Some(FLOODGATE_UUID)),
        ])
        .await;

    assert_eq!(roster.len(), 1);
    assert_eq!(roster[0].get_player_uuid(), Some(FLOODGATE_UUID));
    let steve = report.record("Steve").await.expect("record");
    assert_eq!(report.uuid_owner(FLOODGATE_UUID).await, Some(steve.id));
    assert_eq!(report.uuid_owner(JAVA_UUID_2).await, None);
}

// The Java account already holds the name the beta.27 mod claims for a Floodgate
// player (prefix "", gamertag with a space).
#[tokio::test]
async fn a_java_player_who_shares_the_floodgate_name_keeps_their_record() {
    let report = GameServerReport::new().await;
    report
        .registrar
        .create("Player_One", &Game::Minecraft, Some(JAVA_UUID_2))
        .await
        .expect("java account");

    report
        .ingest(vec![GameServerReport::player(
            "Player_One",
            Some("Player One"),
            Some(FLOODGATE_UUID),
        )])
        .await;

    assert_eq!(report.resolve("Player_One").await, "Player_One");
    assert!(report.sign_in("Player One").await.is_ok());
}

// The same names in the other order: the claim lands first, then the real Java
// account joins.
#[tokio::test]
async fn a_java_player_who_joins_after_a_floodgate_claim_is_not_remapped() {
    let report = GameServerReport::new().await;
    report
        .ingest(vec![GameServerReport::player(
            "Player_One",
            Some("Player One"),
            Some(FLOODGATE_UUID),
        )])
        .await;

    let roster = report
        .ingest(vec![GameServerReport::player("Player_One", None, Some(JAVA_UUID_2))])
        .await;

    assert_eq!(roster[0].get_name(), "Player_One");
    assert!(report.record("Player_One").await.is_some());
}

// Registering the Java player binds their UUID to a record under the alias's name. That
// record is theirs, not proof that the alias is theirs, so later ticks leave them alone.
#[tokio::test]
async fn a_java_player_who_joins_after_a_floodgate_claim_stays_themselves_on_later_ticks() {
    let report = GameServerReport::new().await;
    report
        .ingest(vec![GameServerReport::player(
            "Player_One",
            Some("Player One"),
            Some(FLOODGATE_UUID),
        )])
        .await;

    for _ in 0..3 {
        let roster = report
            .ingest(vec![GameServerReport::player("Player_One", None, Some(JAVA_UUID_2))])
            .await;
        assert_eq!(roster[0].get_name(), "Player_One");
    }
}

// After a restart the registrar's cache is cold, so registering the Xbox player binds their
// UUID to the old record under the linked player's Java name. That must not make the linked
// player's alias theirs.
#[tokio::test]
async fn an_xbox_player_named_like_a_linked_java_name_stays_themselves_after_a_restart() {
    let mut report = GameServerReport::new().await;
    report
        .registrar
        .create("Builder", &Game::Minecraft, Some(JAVA_UUID))
        .await
        .expect("beta.27 orphan");
    report
        .ingest(vec![GameServerReport::player(
            "Builder",
            Some("LongGamertag123"),
            Some(JAVA_UUID),
        )])
        .await;
    report.restart();

    for _ in 0..3 {
        let roster = report
            .ingest(vec![GameServerReport::player("Builder", None, Some(FLOODGATE_UUID_2))])
            .await;
        assert_eq!(roster[0].get_name(), "Builder");
    }
}

// The remap decision is remembered, so a player who follows an alias costs no queries on
// later ticks.
#[tokio::test]
async fn a_remap_decision_is_not_reevaluated_every_tick() {
    let report = GameServerReport::new().await;
    report
        .ingest(vec![GameServerReport::player("JavaName", Some("XboxTag"), Some(JAVA_UUID))])
        .await;
    let first = report
        .ingest(vec![GameServerReport::player("JavaName", None, Some(JAVA_UUID))])
        .await;
    assert_eq!(first[0].get_name(), "XboxTag");
    player_identity::Entity::delete_many()
        .filter(player_identity::Column::Alias.eq(JAVA_UUID))
        .exec(&report.db.connection)
        .await
        .expect("delete uuid row");

    let again = report
        .ingest(vec![GameServerReport::player("JavaName", None, Some(JAVA_UUID))])
        .await;

    assert_eq!(again[0].get_name(), "XboxTag");
}

// beta.27 left a record under a linked player's Java name. That name is also an Xbox
// player's gamertag, and that player must stay themselves.
#[tokio::test]
async fn an_xbox_player_named_like_a_linked_java_name_is_not_remapped() {
    let report = GameServerReport::new().await;
    report
        .registrar
        .create("Builder", &Game::Minecraft, Some(JAVA_UUID))
        .await
        .expect("beta.27 orphan");
    report
        .ingest(vec![GameServerReport::player(
            "Builder",
            Some("LongGamertag123"),
            Some(JAVA_UUID),
        )])
        .await;
    assert_eq!(report.resolve("Builder").await, "LongGamertag123");

    let roster = report
        .ingest(vec![GameServerReport::player("Builder", None, Some(FLOODGATE_UUID_2))])
        .await;

    assert_eq!(roster[0].get_name(), "Builder");
}

// Two gamertags that Floodgate cut to the same Java name, reported by beta.27 mods at
// different times. The alias stays with the first.
#[tokio::test]
async fn a_floodgate_claim_does_not_move_an_alias_owned_by_another_player() {
    let report = GameServerReport::new().await;
    report
        .ingest(vec![GameServerReport::player(
            "BE_LongGamertag1",
            Some("LongGamertag123"),
            Some(FLOODGATE_UUID),
        )])
        .await;

    report
        .ingest(vec![GameServerReport::player(
            "BE_LongGamertag1",
            Some("LongGamertag1XY"),
            Some(FLOODGATE_UUID_2),
        )])
        .await;

    assert_eq!(report.resolve("BE_LongGamertag1").await, "LongGamertag123");
    assert!(report.record("LongGamertag123").await.is_some());
    assert!(report.record("LongGamertag1XY").await.is_some());
}

// Positions arrive several times a second. A claim already recorded is answered from
// memory, not rewritten.
#[tokio::test]
async fn an_accepted_claim_is_not_rewritten_every_tick() {
    let report = GameServerReport::new().await;
    let claim = AliasClaim::new(".Player_One", "Player One", Some(FLOODGATE_UUID));
    report
        .ingest(vec![GameServerReport::player(
            ".Player_One",
            Some("Player One"),
            Some(FLOODGATE_UUID),
        )])
        .await;
    let alias = player_identity::Entity::find()
        .filter(player_identity::Column::Alias.eq(".Player_One"))
        .one(&report.db.connection)
        .await
        .expect("query")
        .expect("alias recorded");
    alias.delete(&report.db.connection).await.expect("delete");

    report
        .identity
        .record_alias_claims(&[claim], &Game::Minecraft)
        .await;

    let again = player_identity::Entity::find()
        .filter(player_identity::Column::Alias.eq(".Player_One"))
        .one(&report.db.connection)
        .await
        .expect("query");
    assert!(again.is_none());
}

// A refused claim is remembered too, so a collision does not cost a query and a warning on
// every tick.
#[tokio::test]
async fn a_refused_claim_is_not_reevaluated_every_tick() {
    let report = GameServerReport::new().await;
    let java = report
        .registrar
        .create("Player_One", &Game::Minecraft, Some(JAVA_UUID_2))
        .await
        .expect("java account");
    let claim = AliasClaim::new("Player_One", "Player One", Some(FLOODGATE_UUID));
    report
        .ingest(vec![GameServerReport::player(
            "Player_One",
            Some("Player One"),
            Some(FLOODGATE_UUID),
        )])
        .await;
    player_identity::Entity::delete_many()
        .filter(player_identity::Column::PlayerId.eq(java.id))
        .exec(&report.db.connection)
        .await
        .expect("delete identities");
    player::Entity::delete_by_id(java.id)
        .exec(&report.db.connection)
        .await
        .expect("delete java account");

    report
        .identity
        .record_alias_claims(&[claim], &Game::Minecraft)
        .await;

    assert_eq!(report.resolve("Player_One").await, "Player_One");
}

struct LinkedJavaName;

impl LinkedJavaName {
    // A linked player whose Java name, "Builder", is recorded as an alias of their gamertag.
    async fn report() -> GameServerReport {
        let report = GameServerReport::new().await;
        report
            .registrar
            .create("Builder", &Game::Minecraft, Some(JAVA_UUID))
            .await
            .expect("beta.27 orphan");
        report
            .ingest(vec![GameServerReport::player(
                "Builder",
                Some("LongGamertag123"),
                Some(JAVA_UUID),
            )])
            .await;
        report
    }
}

// An Xbox player whose gamertag is the linked player's Java name sends a control action.
// It must act on them, not on the linked player.
#[tokio::test]
async fn a_control_action_from_a_stranger_who_shares_an_alias_name_is_not_remapped() {
    let report = LinkedJavaName::report().await;

    let name = report
        .identity
        .canonical_name("Builder", Some(FLOODGATE_UUID_2), &Game::Minecraft)
        .await;

    assert_eq!(name, "Builder");
}

#[tokio::test]
async fn a_control_action_from_the_alias_owner_follows_the_alias() {
    let report = LinkedJavaName::report().await;

    let name = report
        .identity
        .canonical_name("Builder", Some(JAVA_UUID), &Game::Minecraft)
        .await;

    assert_eq!(name, "LongGamertag123");
}

// The BDS addon and older Java mods send no platform UUID. They behave as they always did.
#[tokio::test]
async fn a_control_action_without_a_platform_uuid_follows_the_alias() {
    let report = LinkedJavaName::report().await;

    let name = report
        .identity
        .canonical_name("Builder", None, &Game::Minecraft)
        .await;

    assert_eq!(name, "LongGamertag123");
}

struct MicrosoftVerified;

impl MicrosoftVerified {
    // An admin registered "XboxTag", and that player's sign-in recorded their Java profile
    // name. The game server has never reported them.
    async fn report() -> GameServerReport {
        let report = GameServerReport::new().await;
        let xbox = report
            .registrar
            .create("XboxTag", &Game::Minecraft, None)
            .await
            .expect("admin-created record");
        report
            .identity
            .create_alias(xbox.id, "JavaName", &Game::Minecraft, "minecraft_services")
            .await
            .expect("sign-in alias");
        report
    }
}

// Microsoft vouched for the Java name, so no platform UUID has to be on record first.
#[tokio::test]
async fn a_java_player_verified_by_microsoft_before_their_first_join_is_remapped() {
    let report = MicrosoftVerified::report().await;

    for _ in 0..3 {
        let roster = report
            .ingest(vec![GameServerReport::player("JavaName", None, Some(JAVA_UUID))])
            .await;
        assert_eq!(roster[0].get_name(), "XboxTag");
    }

    assert!(report.record("JavaName").await.is_none());
}

// With an empty Floodgate prefix a Bedrock player can carry the same Java name. Their
// gamertag is their own, whatever Microsoft said about the Java account.
#[tokio::test]
async fn a_floodgate_player_named_like_a_microsoft_verified_java_name_is_not_remapped() {
    let report = MicrosoftVerified::report().await;

    let roster = report
        .ingest(vec![GameServerReport::player("JavaName", None, Some(FLOODGATE_UUID))])
        .await;

    assert_eq!(roster[0].get_name(), "JavaName");
}

// A Java profile name can pass to another account. That account is already a player here
// under its own record, so the name Microsoft once tied to "XboxTag" does not move it.
#[tokio::test]
async fn a_java_account_on_another_record_does_not_follow_a_microsoft_verified_alias() {
    let report = MicrosoftVerified::report().await;
    report
        .registrar
        .create("Alice", &Game::Minecraft, Some(JAVA_UUID_2))
        .await
        .expect("another player's record");

    let roster = report
        .ingest(vec![GameServerReport::player("JavaName", None, Some(JAVA_UUID_2))])
        .await;

    assert_eq!(roster[0].get_name(), "JavaName");
}

// A remembered decision is for one target. When the alias moves to another player, the
// platform UUID that proved the first target proves nothing about the second.
#[tokio::test]
async fn a_moved_alias_is_judged_again_on_the_next_tick() {
    let report = GameServerReport::new().await;
    let first = report
        .registrar
        .create("FirstTag", &Game::Minecraft, Some(JAVA_UUID))
        .await
        .expect("first record");
    let second = report
        .registrar
        .create("SecondTag", &Game::Minecraft, None)
        .await
        .expect("second record");
    report
        .identity
        .create_alias(first.id, "Alias", &Game::Minecraft, "floodgate")
        .await
        .expect("alias");
    let before = report
        .ingest(vec![GameServerReport::player("Alias", None, Some(JAVA_UUID))])
        .await;
    assert_eq!(before[0].get_name(), "FirstTag");

    report
        .identity
        .create_alias(second.id, "Alias", &Game::Minecraft, "floodgate")
        .await
        .expect("moved alias");
    let after = report
        .ingest(vec![GameServerReport::player("Alias", None, Some(JAVA_UUID))])
        .await;

    assert_eq!(after[0].get_name(), "Alias");
}

// The Java account registered the name first. The Bedrock player whose gamertag it is
// arrives on a later tick, and their platform UUID must still reach the record.
#[tokio::test]
async fn a_floodgate_player_who_arrives_after_a_java_player_of_the_same_name_is_bound_to_the_record()
{
    let report = GameServerReport::new().await;
    report
        .ingest(vec![GameServerReport::player("Steve", None, Some(JAVA_UUID_2))])
        .await;

    report
        .ingest(vec![GameServerReport::player("Steve", None, Some(FLOODGATE_UUID))])
        .await;

    let steve = report.record("Steve").await.expect("record");
    assert_eq!(report.uuid_owner(FLOODGATE_UUID).await, Some(steve.id));
}

// The Java account "Steve" links to another gamertag. Their platform UUID is on the record
// named Steve, but so is the UUID of the Xbox player whose gamertag is Steve, and that
// player must not be moved onto the Java player's new gamertag.
#[tokio::test]
async fn a_java_player_cannot_alias_a_name_an_xbox_player_holds() {
    let report = GameServerReport::new().await;
    report
        .ingest(vec![GameServerReport::player("Steve", None, Some(JAVA_UUID_2))])
        .await;
    report
        .ingest(vec![GameServerReport::player("Steve", None, Some(FLOODGATE_UUID))])
        .await;

    report
        .ingest(vec![GameServerReport::player(
            "Steve",
            Some("OtherTag"),
            Some(JAVA_UUID_2),
        )])
        .await;

    assert_eq!(report.resolve("Steve").await, "Steve");
    let roster = report
        .ingest(vec![GameServerReport::player("Steve", None, Some(FLOODGATE_UUID))])
        .await;
    assert_eq!(roster[0].get_name(), "Steve");
}

// A linked player whose Java name ends with their gamertag arrives under the gamertag with
// a Java account UUID. The gamertag sent as its own alternative is Floodgate's word that
// the name is theirs, so a Java account of the same name does not take it.
#[tokio::test]
async fn a_linked_player_who_reports_their_own_name_as_proof_keeps_it_over_a_java_account() {
    let report = GameServerReport::new().await;

    let roster = report
        .ingest(vec![
            GameServerReport::player("Builder", None, Some(JAVA_UUID_2)),
            GameServerReport::player("Builder", Some("Builder"), Some(JAVA_UUID)),
        ])
        .await;

    assert_eq!(roster.len(), 1);
    assert_eq!(roster[0].get_player_uuid(), Some(JAVA_UUID));
}

// A platform UUID is one identity however a game server spells its letters.
#[tokio::test]
async fn a_platform_uuid_in_upper_case_binds_to_the_same_row() {
    let mut report = GameServerReport::new().await;
    report
        .ingest(vec![GameServerReport::player("JavaName", None, Some(JAVA_UUID))])
        .await;
    report.restart();

    report
        .ingest(vec![GameServerReport::player(
            "JavaName",
            None,
            Some(&JAVA_UUID.to_uppercase()),
        )])
        .await;

    let record = report.record("JavaName").await.expect("record");
    let bound = player_identity::Entity::find()
        .filter(player_identity::Column::PlayerId.eq(record.id))
        .filter(player_identity::Column::AliasType.eq("platform_uuid"))
        .all(&report.db.connection)
        .await
        .expect("query");
    assert_eq!(bound.len(), 1);
}
