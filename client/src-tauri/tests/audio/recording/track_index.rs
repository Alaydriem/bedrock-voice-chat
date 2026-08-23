use bvc_client_lib::audio::recording::TrackIndex;
use common::structs::recording::TrackKind;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// A session on disk: a manifest, plus an empty segment file for every key that has audio.
fn session(
    emitter: &str,
    participants: &[&str],
    jukebox: &[&str],
    with_audio: &[&str],
) -> (TempDir, PathBuf) {
    let dir = TempDir::new().expect("temp dir");
    let path = dir.path().join("01J8Z9");
    fs::create_dir_all(path.join("wal")).expect("wal dir");

    let manifest = serde_json::json!({
        "session_id": "01J8Z9",
        "start_timestamp": 1_753_732_440_000u64,
        "end_timestamp": null,
        "duration_ms": 6_128_000u64,
        "emitter_player": emitter,
        "participants": participants,
        "jukebox_participants": jukebox,
        "created_at": "1753732440",
        "recording_version": "1",
        "name": null,
    });
    fs::write(path.join("session.json"), manifest.to_string()).expect("manifest");

    for key in with_audio {
        let sanitized: String = key
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
            .take(20)
            .collect();
        fs::write(path.join("wal").join(format!("{sanitized}-9f2c-0.log")), b"x").expect("segment");
    }

    (dir, path)
}

fn displays(path: &Path) -> Vec<String> {
    TrackIndex::for_session(path)
        .expect("index")
        .into_iter()
        .map(|t| t.display)
        .collect()
}

// The whole point. Your voice is in the WAL under your own name and has never been in
// `participants`, so an index that trusts `participants` alone can never offer it.
#[test]
fn your_own_voice_is_a_track_even_though_no_participant_list_names_it() {
    let (_dir, path) = session(
        "minecraft:Alaydriem",
        &["minecraft:Petra"],
        &[],
        &["minecraft:Alaydriem", "minecraft:Petra"],
    );
    let tracks = TrackIndex::for_session(&path).expect("index");

    assert_eq!(tracks[0].kind, TrackKind::Own);
    assert_eq!(tracks[0].display, "Alaydriem");
}

// A name in a manifest is not proof of a track. Offering one writes an empty file.
#[test]
fn a_session_you_never_spoke_in_offers_no_track_of_yours() {
    let (_dir, path) = session(
        "minecraft:Alaydriem",
        &["minecraft:Petra"],
        &[],
        &["minecraft:Petra"],
    );

    assert_eq!(displays(&path), vec!["Petra".to_string()]);
}

#[test]
fn a_participant_with_no_segments_is_not_offered() {
    let (_dir, path) = session(
        "minecraft:Alaydriem",
        &["minecraft:Petra", "minecraft:Juno"],
        &[],
        &["minecraft:Alaydriem", "minecraft:Petra"],
    );

    assert_eq!(
        displays(&path),
        vec!["Alaydriem".to_string(), "Petra".to_string()]
    );
}

#[test]
fn every_jukebox_source_lands_on_one_track() {
    let (_dir, path) = session(
        "minecraft:Alaydriem",
        &[],
        &["jukebox:rain", "jukebox:sting"],
        &["minecraft:Alaydriem", "jukebox:rain", "jukebox:sting"],
    );
    let tracks = TrackIndex::for_session(&path).expect("index");
    let jukebox = tracks.last().expect("a jukebox track");

    assert_eq!(jukebox.kind, TrackKind::Jukebox);
    assert_eq!(jukebox.display, "Jukebox");
    assert_eq!(jukebox.keys.len(), 2);
}

#[test]
fn no_jukebox_audio_means_no_jukebox_track() {
    let (_dir, path) = session("minecraft:Alaydriem", &[], &[], &["minecraft:Alaydriem"]);
    let tracks = TrackIndex::for_session(&path).expect("index");

    assert!(tracks.iter().all(|t| t.kind != TrackKind::Jukebox));
}

// The recorder is being changed to add the input emitter to `participants` as well, so
// after that change your name is in both places. It is still one track.
#[test]
fn you_appear_once_even_when_the_manifest_names_you_twice() {
    let (_dir, path) = session(
        "minecraft:Alaydriem",
        &["minecraft:Alaydriem", "minecraft:Petra"],
        &[],
        &["minecraft:Alaydriem", "minecraft:Petra"],
    );

    assert_eq!(
        displays(&path),
        vec!["Alaydriem".to_string(), "Petra".to_string()]
    );
}
