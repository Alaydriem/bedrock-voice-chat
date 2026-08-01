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
    // A status panel renders the aggregates above. This is what attributes choppy audio to
    // one speaker, and what the copyable report prints.
    pub peers: Vec<PeerDiagnostics>,
    pub history: Vec<LinkSample>,
}
