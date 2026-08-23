use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub mod failure;
pub mod progress;

pub use failure::ExportFailure;
pub use progress::ExportProgress;

/// What an export actually did.
///
/// A render can fail per track while the others succeed, and a run that reports only
/// success hands somebody a folder with files missing from it and no way to know.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct ExportOutcome {
    pub written: Vec<String>,
    pub failed: Vec<ExportFailure>,
}
