// Emitted once per second while a connection is live, carrying the full link, device and
// per-speaker snapshot. Nothing is emitted while disconnected: a snapshot of zeros renders as a
// healthy link with a 0 ms round trip, which misleads worse than an empty panel.
pub const LINK_DIAGNOSTICS: &str = "link_diagnostics";
