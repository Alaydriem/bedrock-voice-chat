use std::path::PathBuf;

use common::structs::AudioFormat;
use tauri::Emitter;

use crate::audio::recording::TrackSink;
use crate::audio::recording::renderer::{AudioFormatRenderer, ExportNaming, SpatialRenderSettings};
use common::structs::recording::{ExportProgress, RecordingTrack};

/// The run's one track at a time, against the session on disk.
pub struct SessionSink {
    pub app_handle: tauri::AppHandle,
    pub session_id: String,
    pub session_path: PathBuf,
    pub render_path: PathBuf,
    pub format: AudioFormat,
    // Absent when the export is flat. Resolved once for the whole run, because a session that
    // ends part way through must not change the curve the remaining tracks render on.
    pub spatial: Option<SpatialRenderSettings>,
}

#[async_trait::async_trait]
impl TrackSink for SessionSink {
    async fn write(&self, track: &RecordingTrack) -> Result<(), anyhow::Error> {
        let output_path = self.render_path.join(format!(
            "{}.{}",
            ExportNaming::file_stem(track),
            self.format.extension()
        ));
        self.format
            .render_track(
                &self.session_path,
                track,
                &output_path,
                self.spatial.as_ref(),
            )
            .await
    }

    fn progressed(&self, track: &RecordingTrack, index: u32, total: u32) {
        let _ = self.app_handle.emit(
            crate::events::event::RECORDING_EXPORT_PROGRESS,
            ExportProgress {
                session_id: self.session_id.clone(),
                track: track.display.clone(),
                index,
                total,
            },
        );
    }
}
