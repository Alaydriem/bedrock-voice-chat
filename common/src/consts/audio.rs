pub const BUFFER_SIZE: u32 = 960;

pub const SUPPORTED_SAMPLE_RATES: [u32; 2] = [48000, 44100];

pub const JUKEBOX_PLAYER_PREFIX: &str = "jukebox-";

/// The reserved control-plane target that names jukebox music instead of a player.
///
/// A gamertag cannot contain `#`, so this can never collide with a real player. The jukebox
/// rides the per-player preference plane under this name, which is why it needs no packet, no
/// codec entry and no protocol bump of its own — `PlayerPreference { target, volume, muted }`
/// already carries exactly the shape the control needs.
///
/// Distinct from `JUKEBOX_PLAYER_PREFIX`, which keys audio sinks; this keys a preference.
pub const JUKEBOX_CONTROL_TARGET: &str = "#jukebox";
