use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct ExportProgress {
    // Carried so a screen showing one session ignores another session's run.
    pub session_id: String,
    pub track: String,
    // How many are finished, not which one is running: the same number drives a bar.
    pub index: u32,
    pub total: u32,
}
