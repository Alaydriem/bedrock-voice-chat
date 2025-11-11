use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS, Default)]
#[ts(export, export_to = "./../../../src/js/bindings/")]
pub struct OnboardingState {
    pub welcome: bool,
    pub microphone: bool,
    pub notifications: bool,
    pub devices: bool,
}

impl OnboardingState {
    pub fn new() -> Self {
        Self {
            welcome: false,
            microphone: false,
            notifications: false,
            devices: false,
        }
    }

    pub fn is_complete(&self) -> bool {
        self.welcome && self.microphone && self.notifications && self.devices
    }

    /// Returns the first incomplete step, or None if all complete
    pub fn next_incomplete_step(&self) -> Option<&str> {
        if !self.welcome { return Some("welcome"); }
        if !self.microphone { return Some("microphone"); }
        if !self.notifications { return Some("notifications"); }
        if !self.devices { return Some("devices"); }
        None
    }
}
