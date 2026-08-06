pub(crate) mod listener;
pub mod ptt_hold;

pub use listener::KeybindListener;
pub use ptt_hold::PttHold;

// Global shortcuts are a desktop capability. The listener is not: it owns the mute
// transition a voice-mode change requires and the push-to-talk hold itself, both of which
// a phone reaches through a command instead of a hotkey.
#[cfg(desktop)]
mod manager;

#[cfg(desktop)]
pub use manager::{ActionMap, KeybindManager};
