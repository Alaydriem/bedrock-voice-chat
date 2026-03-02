use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS, Default)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct OnboardingState {
    pub welcome: bool,
    pub microphone: bool,
    pub notifications: bool,
    pub devices: bool,
}
