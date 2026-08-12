use bvc_client_lib::audio::recording::renderer::ExportNaming;
use common::structs::recording::{RecordingTrack, TrackKind};

fn track(keys: &[&str], display: &str, kind: TrackKind) -> RecordingTrack {
    RecordingTrack {
        keys: keys.iter().map(|k| k.to_string()).collect(),
        display: display.to_string(),
        kind,
    }
}

// A colon in a filename is an alternate-data-stream separator on NTFS, and a render
// written to one disappears instead of failing.
#[test]
fn a_players_file_is_named_without_the_game_prefix() {
    let stem = ExportNaming::file_stem(&track(
        &["minecraft:Alaydriem"],
        "Alaydriem",
        TrackKind::Player,
    ));

    assert_eq!(stem, "Alaydriem");
    assert!(!stem.contains(':'));
}

#[test]
fn the_jukebox_is_named_for_the_track_and_not_for_a_source() {
    let stem = ExportNaming::file_stem(&track(
        &["jukebox:rain", "jukebox:sting"],
        "Jukebox",
        TrackKind::Jukebox,
    ));

    assert_eq!(stem, "Jukebox");
}

#[test]
fn a_display_name_that_would_break_a_path_is_stripped() {
    let stem = ExportNaming::file_stem(&track(&["k"], "Petra/Juno:1", TrackKind::Player));

    assert!(!stem.contains('/'));
    assert!(!stem.contains(':'));
}
