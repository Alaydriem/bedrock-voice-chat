use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::{
    LinkDiagnostics, LinkSample, MicDiagnostics, PeerDiagnostics, PlaybackDiagnostics,
    SessionDiagnostics,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct LinkDiagnosticsSnapshot {
    pub captured_at_ms: u64,
    pub mic: MicDiagnostics,
    pub playback: PlaybackDiagnostics,
    pub link: LinkDiagnostics,
    pub session: SessionDiagnostics,
    // Level snapshots per second this client publishes on its own push channel.
    //
    // Not a network measurement, and here rather than under any subsystem because it belongs to
    // none of them. It is the rate `LevelEmitPolicy` decides, so it is what separates a change
    // that reduced meter traffic from one that did nothing — and it is the figure that says
    // whether a still meter is being told to be still or is failing to draw what it was told.
    pub meter_events_per_sec: f32,
    // A status panel renders the aggregates above. This is what attributes choppy audio to
    // one speaker, and what the copyable report prints.
    pub peers: Vec<PeerDiagnostics>,
    pub history: Vec<LinkSample>,
}
