// Emitted after the backend mutates the persisted `player_gain_store` (e.g. an
// in-game volume/hear control action), so the dashboard re-reads the store and
// the player cards re-render. Carries no payload — the store is the source of
// truth.
pub(crate) const PLAYER_GAIN_STORE_UPDATED: &str = "player_gain_store_updated";
