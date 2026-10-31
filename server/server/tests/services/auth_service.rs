//! A Bedrock player on a Java server signs in under the name their BVC client knows.
//!
//! The Java mod reports a Floodgate player under the name Floodgate gave them, or, from the
//! current mod, under their gamertag. Their BVC client signs in through Xbox Live, which
//! yields only the gamertag. Both halves must reach the same player record, and no record
//! may be left under the Java name for someone else to take.

use common::traits::player_data::PlayerData;

use crate::harness::{GameServerReport, GateRow, GateRows};

const FLOODGATE_UUID: &str = GateRows::FLOODGATE_UUID;
const JAVA_UUID: &str = GateRows::JAVA_UUID;

struct GateCheck;

impl GateCheck {
    // Everything the gate expects of one row after a roster has been ingested, as a list
    // of what went wrong.
    async fn failures(report: &GameServerReport, row: &GateRow, label: &str) -> Vec<String> {
        let mut failures = Vec::new();

    match report.sign_in(row.gamertag).await {
        Ok(name) if name == row.gamertag => {}
        other => failures.push(format!("{} {label}: sign-in {other:?}", row.id)),
    }

    let Some(record) = report.record(row.gamertag).await else {
        failures.push(format!("{} {label}: no record under the gamertag", row.id));
        return failures;
    };

    let java_name = row.beta27.0;
    if java_name != row.gamertag {
        if report.record(java_name).await.is_some() {
            failures.push(format!("{} {label}: a record exists under {java_name:?}", row.id));
        }
        if report.sign_in(java_name).await.is_ok() {
            failures.push(format!("{} {label}: {java_name:?} signs in", row.id));
        }
    }

    if report.uuid_owner(row.uuid).await != Some(record.id) {
        failures.push(format!("{} {label}: platform UUID bound elsewhere", row.id));
    }

    failures
    }
}

#[tokio::test]
async fn every_gamertag_in_the_gate_signs_in_from_either_mod_version() {
    let mut failures = Vec::new();

    for row in GateRows::ALL {
        for (label, wire) in [("beta27", row.beta27), ("new", row.new)] {
            let report = GameServerReport::new().await;
            report
                .ingest(vec![GameServerReport::player(wire.0, wire.1, Some(row.uuid))])
                .await;
            failures.extend(GateCheck::failures(&report, row, label).await);
        }
    }

    assert!(failures.is_empty(), "{failures:#?}");
}

// The real upgrade: the same database sees the beta.27 mod first, then the current one.
#[tokio::test]
async fn every_gamertag_survives_the_upgrade_on_one_database() {
    let mut failures = Vec::new();

    for row in GateRows::ALL {
        let report = GameServerReport::new().await;
        report
            .ingest(vec![GameServerReport::player(row.beta27.0, row.beta27.1, Some(row.uuid))])
            .await;
        report
            .ingest(vec![GameServerReport::player(row.new.0, row.new.1, Some(row.uuid))])
            .await;
        failures.extend(GateCheck::failures(&report, row, "upgrade").await);
    }

    assert!(failures.is_empty(), "{failures:#?}");
}

// Floodgate's default `replace-spaces` turns "TTV 3RICtion" into ".TTV_3RICtion".
#[tokio::test]
async fn a_floodgate_player_whose_gamertag_has_a_space_can_sign_in() {
    let report = GameServerReport::new().await;

    report
        .ingest(vec![GameServerReport::player(
            ".TTV_3RICtion",
            Some("TTV 3RICtion"),
            Some(FLOODGATE_UUID),
        )])
        .await;

    assert_eq!(
        report.sign_in("TTV 3RICtion").await,
        Ok("TTV 3RICtion".to_string())
    );
}

// Floodgate cuts prefix + gamertag to 16 characters.
#[tokio::test]
async fn a_floodgate_player_whose_name_was_truncated_can_sign_in() {
    let report = GameServerReport::new().await;

    report
        .ingest(vec![GameServerReport::player(
            "BE_LongGamertag1",
            Some("LongGamertag123"),
            Some(FLOODGATE_UUID),
        )])
        .await;

    assert_eq!(
        report.sign_in("LongGamertag123").await,
        Ok("LongGamertag123".to_string())
    );
}

// A linked Floodgate player plays under their Java account name, but their BVC client
// still signs in with the Xbox gamertag.
#[tokio::test]
async fn a_linked_floodgate_player_can_sign_in_with_their_gamertag() {
    let report = GameServerReport::new().await;

    report
        .ingest(vec![GameServerReport::player(
            "JavaName",
            Some("XboxTag"),
            Some(JAVA_UUID),
        )])
        .await;

    assert_eq!(report.sign_in("XboxTag").await, Ok("XboxTag".to_string()));
}

// The BDS addon's shape: the gamertag, no alternative, no platform UUID.
#[tokio::test]
async fn a_bedrock_player_reported_by_gamertag_can_sign_in() {
    let report = GameServerReport::new().await;

    report
        .ingest(vec![GameServerReport::player("TTV 3RICtion", None, None)])
        .await;

    assert_eq!(
        report.sign_in("TTV 3RICtion").await,
        Ok("TTV 3RICtion".to_string())
    );
}

// Records come from the game server only. Signing in never creates one.
#[tokio::test]
async fn a_player_who_never_joined_is_refused() {
    let report = GameServerReport::new().await;

    assert_eq!(
        report.sign_in("Nobody").await,
        Err("player not found in database".to_string())
    );
}

#[tokio::test]
async fn a_bedrock_player_known_from_bds_then_joining_through_floodgate() {
    let report = GameServerReport::new().await;

    report
        .ingest(vec![GameServerReport::player("TTV 3RICtion", None, Some(FLOODGATE_UUID))])
        .await;
    report
        .ingest(vec![GameServerReport::player(
            ".TTV_3RICtion",
            Some("TTV 3RICtion"),
            Some(FLOODGATE_UUID),
        )])
        .await;

    assert!(report.sign_in("TTV 3RICtion").await.is_ok());
    assert!(report.record(".TTV_3RICtion").await.is_none());
}

// A server that ran a beta.27 mod already holds a record under the Floodgate name.
#[tokio::test]
async fn an_orphan_record_from_an_older_mod_resolves_to_the_gamertag() {
    let report = GameServerReport::new().await;
    report
        .registrar
        .create(".TTV_3RICtion", &common::Game::Minecraft, Some(FLOODGATE_UUID))
        .await
        .expect("orphan");

    report
        .ingest(vec![GameServerReport::player(
            ".TTV_3RICtion",
            Some("TTV 3RICtion"),
            Some(FLOODGATE_UUID),
        )])
        .await;

    assert_eq!(
        report.sign_in("TTV 3RICtion").await,
        Ok("TTV 3RICtion".to_string())
    );
    assert_eq!(report.resolve(".TTV_3RICtion").await, "TTV 3RICtion");
}

// After a Floodgate session the server knows the linked player's Java name. When they join
// from Java Edition directly, their positions must land on the gamertag their client uses.
#[tokio::test]
async fn a_linked_player_joining_from_java_edition_is_remapped_to_their_gamertag() {
    let report = GameServerReport::new().await;
    report
        .ingest(vec![GameServerReport::player("JavaName", Some("XboxTag"), Some(JAVA_UUID))])
        .await;

    let roster = report
        .ingest(vec![GameServerReport::player("JavaName", None, Some(JAVA_UUID))])
        .await;

    assert_eq!(roster[0].get_name(), "XboxTag");
}

// A plain Java player who later links: the record made for their Java name is the same
// person, proved by the platform UUID, so the Java name becomes their alias.
#[tokio::test]
async fn a_java_player_who_links_keeps_one_identity() {
    let report = GameServerReport::new().await;
    report
        .ingest(vec![GameServerReport::player("JavaName", None, Some(JAVA_UUID))])
        .await;

    report
        .ingest(vec![GameServerReport::player("JavaName", Some("XboxTag"), Some(JAVA_UUID))])
        .await;

    assert_eq!(report.sign_in("XboxTag").await, Ok("XboxTag".to_string()));
    assert_eq!(report.resolve("JavaName").await, "XboxTag");
}
