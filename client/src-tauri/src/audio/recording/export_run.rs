use async_trait::async_trait;
use common::structs::recording::{ExportFailure, ExportOutcome, RecordingTrack};

/// One track written, and one report that it happened.
///
/// Separating this from the run is what lets the loop be tested without a filesystem, an
/// encoder or an app handle behind it.
#[async_trait]
pub trait TrackSink {
    async fn write(&self, track: &RecordingTrack) -> Result<(), anyhow::Error>;
    fn progressed(&self, track: &RecordingTrack, index: u32, total: u32);
}

/// Every chosen track, in order, whatever happens to any one of them.
pub struct ExportRun;

impl ExportRun {
    pub async fn execute<S: TrackSink + Sync>(
        tracks: &[RecordingTrack],
        sink: &S,
    ) -> ExportOutcome {
        let total = tracks.len() as u32;
        let mut written = Vec::new();
        let mut failed = Vec::new();

        for (index, track) in tracks.iter().enumerate() {
            // A track nobody can read is not a reason to abandon the ones that follow it.
            match sink.write(track).await {
                Ok(()) => written.push(track.display.clone()),
                Err(e) => failed.push(ExportFailure {
                    track: track.display.clone(),
                    reason: e.to_string(),
                }),
            }
            sink.progressed(track, index as u32 + 1, total);
        }

        ExportOutcome { written, failed }
    }
}
