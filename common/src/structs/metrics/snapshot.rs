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
    // Messages per second this client publishes to its own interface for the level meters.
    //
    // Not a network measurement, and here rather than under any subsystem because it belongs to
    // none of them. On Android each of these is a unit of main-thread work — dequeue, marshal a
    // JavaScript string over JNI, evaluate it — on the same thread that paints the meters they
    // feed, so it is the figure that decides whether the meters can keep up at all.
    pub meter_events_per_sec: f32,
    // A status panel renders the aggregates above. This is what attributes choppy audio to
    // one speaker, and what the copyable report prints.
    pub peers: Vec<PeerDiagnostics>,
    pub history: Vec<LinkSample>,
}
