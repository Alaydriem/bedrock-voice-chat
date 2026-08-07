use std::path::Path;
use std::sync::Arc;

use bvc_client_lib::players::{
    PlayerSettings, PlayerSettingsBackend, PlayerSettingsService, RedbBackend,
};
use common::structs::players::PlayerKey;

const SERVER: &str = "bvc.example.com";
const OTHER: &str = "other.example.com";

/// redb holds an exclusive lock on the file, so a second `open` against a path a live store
/// still owns fails with `DatabaseAlreadyOpen`. Every reopen below therefore drops the first
/// service first — which is also the honest shape of the assertion, since what is being
/// checked is what survives the process.
fn open(path: &Path) -> Arc<PlayerSettingsService> {
    PlayerSettingsService::new_shared(PlayerSettings::Redb(
        RedbBackend::open(path).expect("opens"),
    ))
}

fn key(cn: &str) -> PlayerKey {
    PlayerKey::new(SERVER, cn)
}

// Only the two write classes matter here, and they are the whole design: a decision is
// durable at once, a proximity stamp is not worth a disk write per player.
#[test]
fn writes_a_decision_through_immediately() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("a.redb");
    let alaydriem = key("minecraft:Alaydriem");

    let service = open(&path);
    service.set_muted(&alaydriem, true).expect("sets");
    drop(service);

    assert!(open(&path).get(&alaydriem).muted);
}

#[test]
fn holds_a_proximity_stamp_back_until_it_is_flushed() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("b.redb");
    let petra = key("minecraft:Petra");

    let service = open(&path);
    service.touch(&petra);
    assert!(service.dirty(), "a stamp marks the map dirty");
    drop(service);

    let unflushed = open(&path);
    assert!(
        unflushed.get(&petra).last_seen.is_none(),
        "a stamp that was never flushed must not have reached the disk"
    );
    drop(unflushed);

    let service = open(&path);
    service.touch(&petra);
    service.flush().expect("flushes");
    assert!(!service.dirty(), "flushing clears the flag");
    drop(service);

    assert!(open(&path).get(&petra).last_seen.is_some());
}

// A player nobody has an opinion about reads as unity gain rather than as an error, which
// is what lets the projection be rebuilt without a branch per absent player.
#[test]
fn reads_a_default_for_a_player_it_has_never_seen() {
    let dir = tempfile::tempdir().expect("temp dir");
    let service = open(&dir.path().join("c.redb"));

    let settings = service.get(&key("minecraft:Nobody"));
    assert_eq!(settings.gain, 1.0);
    assert!(!settings.muted);
}

// Reset is for "I cannot hear somebody and do not remember muting them". It clears the
// decisions and keeps the people, so the list does not empty itself.
#[test]
fn reset_clears_decisions_and_keeps_the_rows() {
    let dir = tempfile::tempdir().expect("temp dir");
    let service = open(&dir.path().join("d.redb"));
    let alaydriem = key("minecraft:Alaydriem");

    service.set_muted(&alaydriem, true).expect("sets");
    service.reset_all(SERVER).expect("resets");

    assert!(!service.get(&alaydriem).muted);
    assert_eq!(service.rows(SERVER).len(), 1);
}

// Reset is scoped to the server you are on. Clearing your mutes here must not clear them on
// a server you are not looking at and cannot see the result on.
#[test]
fn reset_leaves_another_server_alone() {
    let dir = tempfile::tempdir().expect("temp dir");
    let service = open(&dir.path().join("g.redb"));
    let here = key("minecraft:Alaydriem");
    let there = PlayerKey::new(OTHER, "minecraft:Alaydriem");

    service.set_muted(&here, true).expect("sets");
    service.set_muted(&there, true).expect("sets");
    service.reset_all(SERVER).expect("resets");

    assert!(!service.get(&here).muted);
    assert!(service.get(&there).muted);
}

// Forget is "stop showing me this person", which is a different request from reset.
#[test]
fn forget_removes_the_row_entirely() {
    let dir = tempfile::tempdir().expect("temp dir");
    let service = open(&dir.path().join("e.redb"));
    let alaydriem = key("minecraft:Alaydriem");

    service.set_muted(&alaydriem, true).expect("sets");
    service.forget(&alaydriem).expect("forgets");

    assert!(service.rows(SERVER).is_empty());
}

// A stamp for somebody already carrying a decision must not overwrite the decision.
#[test]
fn a_proximity_stamp_leaves_a_decision_alone() {
    let dir = tempfile::tempdir().expect("temp dir");
    let service = open(&dir.path().join("f.redb"));
    let alaydriem = key("minecraft:Alaydriem");

    service.set_gain(&alaydriem, 0.4).expect("sets");
    service.touch(&alaydriem);

    assert_eq!(service.get(&alaydriem).gain, 0.4);
}

// What the mixer receives. The server is dropped on the way out because GainProjection is
// keyed on identity alone — and a row from another server must not survive that drop, or
// one server's mute silences the same player everywhere.
#[test]
fn projects_only_the_current_servers_rows_for_the_mixer() {
    let dir = tempfile::tempdir().expect("temp dir");
    let service = open(&dir.path().join("h.redb"));

    service.set_muted(&key("minecraft:Alaydriem"), true).expect("sets");
    service
        .set_muted(&PlayerKey::new(OTHER, "minecraft:Petra"), true)
        .expect("sets");

    let projected = service.store_for(SERVER);
    assert_eq!(projected.0.len(), 1);
    assert!(projected.0.get("minecraft:Alaydriem").expect("present").muted);
    assert!(projected.0.get("minecraft:Petra").is_none());
}

/// Proximity writes a row for everybody who walks past, on every server, forever. Without a
/// pruner the file grows without bound and almost every row in it is at unity gain.
mod pruning {
    use super::*;
    use common::structs::audio::PlayerGainSettings;
    use common::structs::players::PlayerSettingsRow;

    const DAY_MS: f64 = 86_400_000.0;

    fn at(now: f64, days_ago: f64) -> Option<f64> {
        Some(now - days_ago * DAY_MS)
    }

    fn now_millis() -> f64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_millis() as f64
    }

    fn seeded(path: &std::path::Path, rows: &[PlayerSettingsRow]) {
        let store = RedbBackend::open(path).expect("opens");
        store.write_all(rows).expect("writes");
    }

    fn row(cn: &str, gain: f32, muted: bool, last_seen: Option<f64>) -> PlayerSettingsRow {
        PlayerSettingsRow {
            key: PlayerKey::new(SERVER, cn),
            settings: PlayerGainSettings {
                gain,
                muted,
                last_seen,
            },
        }
    }

    #[test]
    fn drops_a_stale_row_that_carries_no_decision() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("p1.redb");
        let now = now_millis();
        seeded(&path, &[row("minecraft:Stranger", 1.0, false, at(now, 40.0))]);

        assert!(open(&path).rows(SERVER).is_empty());
    }

    #[test]
    fn keeps_a_recent_row_that_carries_no_decision() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("p2.redb");
        let now = now_millis();
        seeded(&path, &[row("minecraft:Neighbour", 1.0, false, at(now, 1.0))]);

        assert_eq!(open(&path).rows(SERVER).len(), 1);
    }

    // The whole point of the store. A volume you set is worth keeping forever; a record that
    // somebody walked past you in March is not.
    #[test]
    fn keeps_a_decision_however_old() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("p3.redb");
        let now = now_millis();
        seeded(
            &path,
            &[
                row("minecraft:Muted", 1.0, true, at(now, 400.0)),
                row("minecraft:Quiet", 0.3, false, at(now, 400.0)),
            ],
        );

        assert_eq!(open(&path).rows(SERVER).len(), 2);
    }

    // A mute set on somebody you never walked past has no stamp at all. Treating an absent
    // stamp as "infinitely old" would delete exactly the rows the user cared most about.
    #[test]
    fn keeps_a_decision_that_was_never_stamped() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("p4.redb");
        seeded(&path, &[row("minecraft:Muted", 1.0, true, None)]);

        assert_eq!(open(&path).rows(SERVER).len(), 1);
    }

    // An unstamped row carrying no decision records nothing anybody chose, and no amount of
    // time can make it older than it already is.
    #[test]
    fn drops_an_unstamped_row_that_carries_no_decision() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("p5.redb");
        seeded(&path, &[row("minecraft:Ghost", 1.0, false, None)]);

        assert!(open(&path).rows(SERVER).is_empty());
    }

    // Pruning happens at load, so it has to reach the file and not just the map — otherwise
    // the next launch pays for the same rows again.
    #[test]
    fn the_prune_reaches_the_file() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("p6.redb");
        let now = now_millis();
        seeded(
            &path,
            &[
                row("minecraft:Stranger", 1.0, false, at(now, 40.0)),
                row("minecraft:Quiet", 0.3, false, at(now, 40.0)),
            ],
        );

        drop(open(&path));

        let rows = RedbBackend::open(&path)
            .expect("reopens")
            .load_all()
            .expect("reads");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].key.cn, "minecraft:Quiet");
    }
}

// The degraded path. A file that will not open is transient, so the session runs in memory:
// audio still applies whatever the user sets, and startup does not fail.
#[test]
fn a_memory_only_service_still_serves_reads_and_writes() {
    let service = PlayerSettingsService::new_memory_only();
    let alaydriem = key("minecraft:Alaydriem");

    service.set_gain(&alaydriem, 0.3).expect("sets");
    assert_eq!(service.get(&alaydriem).gain, 0.3);
    service.flush().expect("flush is a no-op, not an error");
}
