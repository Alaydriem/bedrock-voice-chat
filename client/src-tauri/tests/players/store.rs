use bvc_client_lib::players::{PlayerSettingsBackend, RedbBackend};
use common::structs::audio::PlayerGainSettings;
use common::structs::players::{PlayerKey, PlayerSettingsRow};

fn row(server: &str, cn: &str, gain: f32, muted: bool) -> PlayerSettingsRow {
    PlayerSettingsRow {
        key: PlayerKey::new(server, cn),
        settings: PlayerGainSettings {
            gain,
            muted,
            last_seen: Some(1_753_732_440_000.0),
        },
    }
}

fn store(name: &str) -> (RedbBackend, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("temp dir");
    let store = RedbBackend::open(&dir.path().join(name)).expect("opens");
    (store, dir)
}

#[test]
fn writes_and_reads_back_every_row() {
    let (store, _dir) = store("a.redb");
    store
        .write_all(&[
            row("bvc.example.com", "minecraft:Alaydriem", 1.45, false),
            row("bvc.example.com", "minecraft:Petra", 1.0, true),
        ])
        .expect("writes");
    assert_eq!(store.load_all().expect("reads").len(), 2);
}

// The same player on two servers is two rows, which is the whole point of the composite
// key. Keying on the name alone would make one server's decision leak into the other.
#[test]
fn keeps_the_same_player_on_two_servers_apart() {
    let (store, _dir) = store("b.redb");
    store
        .write_all(&[
            row("a.example.com", "minecraft:Alaydriem", 0.5, false),
            row("b.example.com", "minecraft:Alaydriem", 1.0, true),
        ])
        .expect("writes");

    let rows = store.load_all().expect("reads");
    assert_eq!(rows.len(), 2);
    let muted_on = |host: &str| {
        rows.iter()
            .find(|r| r.key.server == host)
            .map(|r| r.settings.muted)
            .unwrap_or(false)
    };
    assert!(!muted_on("a.example.com"));
    assert!(muted_on("b.example.com"));
}

// Survives the process, which is the only reason this file exists.
#[test]
fn survives_a_reopen() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("c.redb");
    RedbBackend::open(&path)
        .expect("opens")
        .write_all(&[row("bvc.example.com", "minecraft:Alaydriem", 0.25, false)])
        .expect("writes");

    let reopened = RedbBackend::open(&path).expect("reopens");
    assert_eq!(reopened.load_all().expect("reads")[0].settings.gain, 0.25);
}

// A mute for somebody you never walked past has `last_seen: None`, and that field carries
// `skip_serializing_if`, so it is absent from the encoded value entirely. A non-self-describing
// encoding cannot read that back — it runs off the end of the buffer, the row fails to decode,
// and the mute silently disappears on the next launch. This is the row shape that catches it.
#[test]
fn survives_a_decision_that_carries_no_proximity_stamp() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("g.redb");
    let key = PlayerKey::new("bvc.example.com", "minecraft:Alaydriem");

    let store = RedbBackend::open(&path).expect("opens");
    store
        .write_all(&[PlayerSettingsRow {
            key: key.clone(),
            settings: PlayerGainSettings {
                gain: 1.0,
                muted: true,
                last_seen: None,
            },
        }])
        .expect("writes");
    drop(store);

    let rows = RedbBackend::open(&path)
        .expect("reopens")
        .load_all()
        .expect("reads");
    assert_eq!(rows.len(), 1, "a row with no last_seen must survive a reopen");
    assert!(rows[0].settings.muted);
    assert!(rows[0].settings.last_seen.is_none());
}

// redb is ACID. A clean crash cannot leave a torn write, so a file that will not parse is a
// bug, a hardware fault, or a file from a newer version. None of those is a reason to delete
// the user's data. The file is moved aside, so it can be inspected and recovered.
#[test]
fn moves_an_unreadable_file_aside_and_keeps_it() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("e.redb");
    std::fs::write(&path, b"this is not a redb database").expect("writes garbage");

    let store = RedbBackend::open(&path).expect("recovers");
    assert!(store.load_all().expect("reads").is_empty());

    let kept: Vec<_> = std::fs::read_dir(dir.path())
        .expect("reads dir")
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().contains("corrupt"))
        .collect();
    assert_eq!(kept.len(), 1, "the unreadable file must be kept, not deleted");
}

// On Windows redb reads its header with a `seek_read` loop that treats the `Ok(0)` at
// end-of-file as "keep going", so a file too short to hold the header spins a core forever
// instead of returning an error — inside app setup, with no window and no log line. The only
// remedy a user has is deleting a file they do not know exists. The test would hang rather than
// fail if this regressed, which is itself the signal.
#[test]
fn does_not_hang_on_a_file_too_short_to_be_a_database() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("short.redb");
    // redb's own magic, then nothing. Without the length guard this is the input that spins.
    std::fs::write(&path, b"redb1\x1A\x0D\x0A").expect("writes a short file");

    let store = RedbBackend::open(&path).expect("recovers");
    assert!(store.load_all().expect("reads").is_empty());

    let kept = std::fs::read_dir(dir.path())
        .expect("reads dir")
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().contains("corrupt"))
        .count();
    assert_eq!(kept, 1, "the short file must be kept, not deleted");
}

// A real database truncated part-way through makes redb assert rather than return an error, and
// an unwind out of the setup hook stops the app starting at all. Disjoint from the case above:
// this file is far longer than a header, and that one never panics.
#[test]
fn survives_a_database_truncated_part_way_through() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("half.redb");

    let store = RedbBackend::open(&path).expect("opens");
    store
        .write_all(&[row("bvc.example.com", "minecraft:Alaydriem", 0.5, false)])
        .expect("writes");
    drop(store);

    let full = std::fs::metadata(&path).expect("stats").len();
    assert!(full > 320, "a real database is far longer than its header");
    let file = std::fs::OpenOptions::new()
        .write(true)
        .open(&path)
        .expect("opens for truncation");
    file.set_len(full / 2).expect("truncates");
    drop(file);

    let store = RedbBackend::open(&path).expect("recovers instead of panicking");
    assert!(store.load_all().expect("reads").is_empty());
}

// A file that is locked, or that this process cannot read, is a transient condition. Deleting
// it would destroy good data because of a permission error or an antivirus scan. The path is
// built by nesting under a regular file, which cannot be a directory on either platform — a
// hardcoded `/definitely/not/a/dir` resolves to a creatable `C:\` path on Windows and would
// assert the wrong thing.
#[test]
fn refuses_to_destroy_a_file_it_merely_cannot_open() {
    let dir = tempfile::tempdir().expect("temp dir");
    let blocker = dir.path().join("not-a-dir");
    std::fs::write(&blocker, b"x").expect("writes blocker");

    let outcome = RedbBackend::open(&blocker.join("x.redb"));
    assert!(outcome.is_err(), "an unopenable path is an error, not a reset");
    assert!(
        blocker.is_file(),
        "the blocking file must be left exactly as it was"
    );
}

