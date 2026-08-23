use serde::{Deserialize, Serialize};
use ts_rs::TS;

// The device screens that follow sign-in. `welcome` and `privacy` are not here:
// the four-step introduction replaces the first and the sign-in footbar's privacy
// link replaces the second.
#[derive(Debug, Clone, Serialize, Deserialize, TS, Default)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct SetupState {
    pub microphone: bool,
    pub notifications: bool,
    pub devices: bool,
}
