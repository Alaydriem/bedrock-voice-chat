use common::structs::audio::AudioDeviceType;
use std::time::Duration;

mod device_state;
mod verdict;

use device_state::DeviceState;
pub use verdict::RebuildVerdict;

/// How many times a failing audio stream is rebuilt, and how far apart.
///
/// A device that refuses to open refuses identically every time. Two mechanisms used to retry
/// it — the webview's recovery listener at roughly 2 Hz and the capture watchdog at 3 s — and
/// neither had a stopping condition, so a single unusable endpoint produced `ERROR` records for
/// as long as the application ran. Sentry captures `ERROR` as an event, which is how one
/// machine burned a month's budget.
///
/// Deliberately not a rate limit. A rate limit makes a permanent fault cheaper; it does not make
/// it finite. What is needed is an end, and something for the user to see once it is reached.
pub struct RebuildBreaker {
    input: DeviceState,
    output: DeviceState,
}

impl RebuildBreaker {
    /// Rebuild attempts before the breaker opens.
    ///
    /// Five doublings from one second span about half a minute, which covers the faults that
    /// genuinely are transient — a device settling after a sample-rate change, an exclusive-mode
    /// application closing — without letting a permanent one run past the point where more
    /// evidence is arriving.
    pub(crate) const MAX_ATTEMPTS: u32 = 5;

    /// Delay before the first rebuild. Each subsequent one doubles it.
    const FIRST_DELAY: Duration = Duration::from_secs(1);

    pub fn new() -> Self {
        Self {
            input: DeviceState::default(),
            output: DeviceState::default(),
        }
    }

    /// A rebuild failed. Returns whether to try again, and how long to wait.
    pub fn observe_failure(&mut self, device: &AudioDeviceType) -> RebuildVerdict {
        let state = self.state_mut(device);

        if state.open || state.attempts >= Self::MAX_ATTEMPTS {
            state.open = true;
            return RebuildVerdict::Open;
        }

        let attempt = state.attempts + 1;
        state.attempts = attempt;

        RebuildVerdict::Retry {
            after: Self::FIRST_DELAY * 2u32.pow(attempt - 1),
            attempt,
        }
    }

    /// A stream built. The next failure starts a fresh episode rather than continuing this one.
    pub fn observe_success(&mut self, device: &AudioDeviceType) {
        *self.state_mut(device) = DeviceState::default();
    }

    /// Something changed that could make the device openable — a different device chosen, the
    /// audio stack reset. An open breaker is cleared only from here and from a success, never
    /// on a timer: a timer would resume paying for a fault that is permanent.
    pub fn rearm(&mut self, device: &AudioDeviceType) {
        self.observe_success(device);
    }

    pub fn is_open(&self, device: &AudioDeviceType) -> bool {
        match device {
            AudioDeviceType::InputDevice => self.input.open,
            AudioDeviceType::OutputDevice => self.output.open,
        }
    }

    fn state_mut(&mut self, device: &AudioDeviceType) -> &mut DeviceState {
        match device {
            AudioDeviceType::InputDevice => &mut self.input,
            AudioDeviceType::OutputDevice => &mut self.output,
        }
    }
}

impl Default for RebuildBreaker {
    fn default() -> Self {
        Self::new()
    }
}
