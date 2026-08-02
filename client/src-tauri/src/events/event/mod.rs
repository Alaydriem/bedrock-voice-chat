pub(crate) mod channel_event;
pub(crate) mod notification;
pub mod player_gain_store;
pub(crate) mod player_presence;
pub(crate) mod server_error;

// Emitted once per second while a connection is live, carrying the full link, device and
// per-speaker snapshot. Nothing is emitted while disconnected: a snapshot of zeros renders as a
// healthy link with a 0 ms round trip, which misleads worse than an empty panel.
//
// Declared here rather than in its own module because the payload it carries,
// `LinkDiagnosticsSnapshot`, lives in `common` so ts-rs can export it — leaving the module with
// nothing but this line.
pub const LINK_DIAGNOSTICS: &str = "link_diagnostics";
