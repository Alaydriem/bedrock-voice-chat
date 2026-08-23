use super::{input, output};

// Live mute state, readable without locking the audio manager. Both flags are process-global
// already; this exists so a diagnostic can observe them without reaching into the private stream
// modules.
pub(crate) struct MuteFlags;

impl MuteFlags {
    pub(crate) fn input_muted() -> bool {
        input::MUTE_INPUT_STREAM.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub(crate) fn output_muted() -> bool {
        output::MUTE_OUTPUT_STREAM.load(std::sync::atomic::Ordering::Relaxed)
    }

    // Test-only setters. The flags are process-global and normally moved by a keybind, the UI, an
    // in-game command or a WebSocket client; without these a test can only observe the default and
    // so cannot tell a wired field from a hardcoded one.
    #[cfg(any(test, feature = "e2e"))]
    pub(crate) fn set_input_muted(muted: bool) {
        input::MUTE_INPUT_STREAM.store(muted, std::sync::atomic::Ordering::Relaxed);
    }

    #[cfg(any(test, feature = "e2e"))]
    pub(crate) fn set_output_muted(muted: bool) {
        output::MUTE_OUTPUT_STREAM.store(muted, std::sync::atomic::Ordering::Relaxed);
    }
}
