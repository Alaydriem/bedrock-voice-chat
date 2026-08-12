use common::structs::recording::RecordingTrack;

/// What a rendered track is called on disk.
///
/// A person opens these files, so they are named for the track rather than for the key
/// behind it. The characters removed here are the ones that make a path mean something
/// else: a colon opens an alternate data stream on NTFS, and a separator plants the file
/// in a directory that was never created.
pub struct ExportNaming;

impl ExportNaming {
    pub fn file_stem(track: &RecordingTrack) -> String {
        track
            .display
            .chars()
            .filter(|c| !matches!(c, ':' | '/' | '\\' | '<' | '>' | '"' | '|' | '?' | '*'))
            .collect()
    }
}
