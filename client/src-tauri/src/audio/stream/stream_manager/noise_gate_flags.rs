use super::input;

// Whether the noise gate is bound to the capture path, readable without locking the audio
// manager. Exists for the same reason as `MuteFlags`: a diagnostic has to observe the flag
// the audio path actually reads, not the copy the settings screen holds. The two disagreeing
// is precisely the fault this reports on.
pub(crate) struct NoiseGateFlags;

impl NoiseGateFlags {
    pub(crate) fn enabled() -> bool {
        input::USE_NOISE_GATE.load(std::sync::atomic::Ordering::Relaxed)
    }

    // Test-only setter, for the same reason as the mute ones: without it a test can only
    // observe the default and cannot tell a wired field from a hardcoded one.
    #[cfg(any(test, feature = "e2e"))]
    pub(crate) fn set_enabled(enabled: bool) {
        input::USE_NOISE_GATE.store(enabled, std::sync::atomic::Ordering::Relaxed);
    }
}
