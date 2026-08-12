// Emitted after the backend flips `jukebox_muted` — an in-game control, a WebSocket toggle, or
// the settings pane itself — so every surface that draws the state re-reads it. The payload is
// the new value; the store remains the source of truth.
pub const JUKEBOX_MUTED_UPDATED: &str = "jukebox_muted_updated";

// Emitted after the backend writes `jukebox_gain` — an in-game control, a WebSocket command, or
// the settings pane itself — so every surface that draws the level re-reads it. The payload is
// the new fraction; the store remains the source of truth.
pub const JUKEBOX_GAIN_UPDATED: &str = "jukebox_gain_updated";
