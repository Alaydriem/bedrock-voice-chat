use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct ExportFailure {
    // The track's display name, because that is what the person picked.
    pub track: String,
    pub reason: String,
}
