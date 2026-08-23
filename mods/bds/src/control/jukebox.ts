// The jukebox rides the per-player preference plane as a reserved target rather than carrying
// actions of its own, so these two values are the whole of its wire contract. Both must match the
// Rust side exactly; this pack shares no types with the `common` crate by design, and a drift here
// silently adjusts nobody.

// Matches common's `JUKEBOX_CONTROL_TARGET`. A gamertag cannot contain '#', so this can never
// collide with a real player.
export const JUKEBOX_TARGET = '#jukebox';

// The loudest anything plays, as a percent. Matches common's `PlayerGainSettings::MAX_GAIN`.
export const MAX_LEVEL = 150;
