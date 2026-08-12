use std::fs;
use std::path::Path;

use common::structs::recording::{RecordingTrack, SessionManifest, TrackKind};

use crate::audio::recording::WalKey;

/// What a session can actually write.
///
/// Built from the manifest and the segment files together, because neither alone is the
/// answer: the manifest names people it received audio from and never names you, and a
/// name in it is not proof that any audio was captured under that name.
pub struct TrackIndex;

impl TrackIndex {
    const JUKEBOX_DISPLAY: &'static str = "Jukebox";

    pub fn for_session(session_path: &Path) -> Result<Vec<RecordingTrack>, anyhow::Error> {
        let manifest: SessionManifest =
            serde_json::from_str(&fs::read_to_string(session_path.join("session.json"))?)?;
        let segments = Self::segments(&session_path.join("wal"));

        let mut tracks = Vec::new();

        if Self::has_audio(&segments, &manifest.emitter_player) {
            tracks.push(Self::voice(&manifest.emitter_player, TrackKind::Own));
        }

        for name in &manifest.participants {
            if *name == manifest.emitter_player || !Self::has_audio(&segments, name) {
                continue;
            }
            tracks.push(Self::voice(name, TrackKind::Player));
        }

        let jukebox: Vec<String> = manifest
            .jukebox_participants
            .iter()
            .filter(|name| Self::has_audio(&segments, name))
            .cloned()
            .collect();
        if !jukebox.is_empty() {
            tracks.push(RecordingTrack {
                keys: jukebox,
                display: Self::JUKEBOX_DISPLAY.to_string(),
                kind: TrackKind::Jukebox,
            });
        }

        Ok(tracks)
    }

    fn voice(identity: &str, kind: TrackKind) -> RecordingTrack {
        RecordingTrack {
            keys: vec![identity.to_string()],
            display: common::Game::display_name(identity).to_string(),
            kind,
        }
    }

    /// One directory read for the whole session. A read per candidate would scale with the
    /// cast on a path that runs every time a session is opened.
    fn segments(wal_path: &Path) -> Vec<String> {
        let Ok(entries) = fs::read_dir(wal_path) else {
            return Vec::new();
        };
        entries
            .flatten()
            .filter_map(|entry| entry.file_name().to_str().map(str::to_string))
            .collect()
    }

    fn has_audio(segments: &[String], key: &str) -> bool {
        segments.iter().any(|name| WalKey::matches(name, key))
    }
}
