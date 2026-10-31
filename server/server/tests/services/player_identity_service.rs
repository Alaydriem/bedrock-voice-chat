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
        ".TTV_3RICtion",
        Some("TTV 3RICtion"),
        Some(FLOODGATE_UUID),
    )];

    let claims = report
        .identity
        .canonicalize_roster(&mut roster, &Game::Minecraft)
        .await;

    assert_eq!(roster[0].get_name(), "TTV 3RICtion");
    assert_eq!(roster[0].get_alternative_identity(), None);
    assert_eq!(
        claims,
        vec![AliasClaim::new(".TTV_3RICtion", "TTV 3RICtion", Some(FLOODGATE_UUID))]
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

#[tokio::test]
async fn an_alternative_identity_equal_to_the_name_makes_no_claim() {
    let report = GameServerReport::new().await;
    let mut roster = vec![GameServerReport::player("XboxTag", Some("XboxTag"), Some(JAVA_UUID))];

    let claims = report
        .identity
        .canonicalize_roster(&mut roster, &Game::Minecraft)
        .await;

    assert_eq!(roster[0].get_name(), "XboxTag");
    assert!(claims.is_empty());
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
        .create("TTV_3RICtion", &Game::Minecraft, Some(JAVA_UUID_2))
        .await
        .expect("java account");

    report
        .ingest(vec![GameServerReport::player(
            "TTV_3RICtion",
            Some("TTV 3RICtion"),
            Some(FLOODGATE_UUID),
        )])
        .await;

    assert_eq!(report.resolve("TTV_3RICtion").await, "TTV_3RICtion");
    assert!(report.sign_in("TTV 3RICtion").await.is_ok());
}

// The same names in the other order: the claim lands first, then the real Java
// account joins.
#[tokio::test]
async fn a_java_player_who_joins_after_a_floodgate_claim_is_not_remapped() {
    let report = GameServerReport::new().await;
    report
        .ingest(vec![GameServerReport::player(
            "TTV_3RICtion",
            Some("TTV 3RICtion"),
            Some(FLOODGATE_UUID),
        )])
        .await;

    let roster = report
        .ingest(vec![GameServerReport::player("TTV_3RICtion", None, Some(JAVA_UUID_2))])
        .await;

    assert_eq!(roster[0].get_name(), "TTV_3RICtion");
    assert!(report.record("TTV_3RICtion").await.is_some());
}

// Registering the Java player binds their UUID to a record under the alias's name. That
// record is theirs, not proof that the alias is theirs, so later ticks leave them alone.
#[tokio::test]
async fn a_java_player_who_joins_after_a_floodgate_claim_stays_themselves_on_later_ticks() {
    let report = GameServerReport::new().await;
    report
        .ingest(vec![GameServerReport::player(
            "TTV_3RICtion",
            Some("TTV 3RICtion"),
            Some(FLOODGATE_UUID),
        )])
        .await;

    for _ in 0..3 {
        let roster = report
            .ingest(vec![GameServerReport::player("TTV_3RICtion", None, Some(JAVA_UUID_2))])
            .await;
        assert_eq!(roster[0].get_name(), "TTV_3RICtion");
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
        .create("Notch", &Game::Minecraft, Some(JAVA_UUID))
        .await
        .expect("beta.27 orphan");
    report
        .ingest(vec![GameServerReport::player(
            "Notch",
            Some("LongGamertag123"),
            Some(JAVA_UUID),
        )])
        .await;
    report.restart();

    for _ in 0..3 {
        let roster = report
            .ingest(vec![GameServerReport::player("Notch", None, Some(FLOODGATE_UUID_2))])
            .await;
        assert_eq!(roster[0].get_name(), "Notch");
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
        .create("Notch", &Game::Minecraft, Some(JAVA_UUID))
        .await
        .expect("beta.27 orphan");
    report
        .ingest(vec![GameServerReport::player(
            "Notch",
            Some("LongGamertag123"),
            Some(JAVA_UUID),
        )])
        .await;
    assert_eq!(report.resolve("Notch").await, "LongGamertag123");

    let roster = report
        .ingest(vec![GameServerReport::player("Notch", None, Some(FLOODGATE_UUID_2))])
        .await;

    assert_eq!(roster[0].get_name(), "Notch");
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
    let claim = AliasClaim::new(".TTV_3RICtion", "TTV 3RICtion", Some(FLOODGATE_UUID));
    report
        .ingest(vec![GameServerReport::player(
            ".TTV_3RICtion",
            Some("TTV 3RICtion"),
            Some(FLOODGATE_UUID),
        )])
        .await;
    let alias = player_identity::Entity::find()
        .filter(player_identity::Column::Alias.eq(".TTV_3RICtion"))
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
        .filter(player_identity::Column::Alias.eq(".TTV_3RICtion"))
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
        .create("TTV_3RICtion", &Game::Minecraft, Some(JAVA_UUID_2))
        .await
        .expect("java account");
    let claim = AliasClaim::new("TTV_3RICtion", "TTV 3RICtion", Some(FLOODGATE_UUID));
    report
        .ingest(vec![GameServerReport::player(
            "TTV_3RICtion",
            Some("TTV 3RICtion"),
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

    assert_eq!(report.resolve("TTV_3RICtion").await, "TTV_3RICtion");
}
