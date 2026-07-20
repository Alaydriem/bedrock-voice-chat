#[cfg(feature = "e2e")]
use crate::audio::stream::stream_manager::sink::CapturingSink;
#[cfg(feature = "e2e")]
use crate::audio::stream::stream_manager::source::BridgeInputSource;

/// Selects the audio backend used when wiring the managed application state.
/// `Real` selects the production Cpal input / Rodio output backends; `Fake`
/// injects a bridge input source and a capturing sink for the test harness so
/// the same construction path is exercised in both the real `run()` and tests.
pub enum AudioBackend {
    Real,
    #[cfg(feature = "e2e")]
    Fake {
        input: BridgeInputSource,
        capture: CapturingSink,
    },
}
