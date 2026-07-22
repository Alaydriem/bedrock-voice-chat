// Emitted after the backend mutates the persisted `player_gain_store` (e.g. an
// in-game volume/hear control action), so the dashboard re-reads the store and
// the player cards re-render. The payload is the canonical target name (for
// device-log tracing); the store remains the source of truth.
pub const PLAYER_GAIN_STORE_UPDATED: &str = "player_gain_store_updated";
