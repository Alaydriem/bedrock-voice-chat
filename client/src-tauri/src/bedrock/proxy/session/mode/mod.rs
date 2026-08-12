mod full;
mod relay_only;

pub use full::FullDispatch;
pub use relay_only::RelayOnlyDispatch;

use common::bedrock_protocol::Event;

use super::BedrockSessionState;
use super::DispatchOutcome;

// What a child dispatcher decided: whether the session continues, and whether
// the session state moved. The caller needs both — it owns the cache write a
// state change implies, and only it can end the session.
//
// Generic over both payloads so a richer state-delta can replace the flag
// without a second container type.
pub struct DispatchResult<O, S> {
    pub outcome: O,
    pub state_changed: S,
}

pub trait ModeDispatch {
    fn dispatch(
        &mut self,
        evt: &Event,
        state: &mut BedrockSessionState,
    ) -> DispatchResult<DispatchOutcome, bool>;
}

// Holds whichever child the resolved mode selected. An enum rather than a boxed
// trait object, and built once so a child's per-session trackers survive across
// events.
pub enum ModeDispatcher {
    RelayOnly(RelayOnlyDispatch),
    Full(FullDispatch),
}

impl ModeDispatch for ModeDispatcher {
    fn dispatch(
        &mut self,
        evt: &Event,
        state: &mut BedrockSessionState,
    ) -> DispatchResult<DispatchOutcome, bool> {
        match self {
            Self::RelayOnly(d) => d.dispatch(evt, state),
            Self::Full(d) => d.dispatch(evt, state),
        }
    }
}
