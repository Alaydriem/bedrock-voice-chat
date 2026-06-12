use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
#[serde(rename_all = "snake_case")]
pub enum GateReason {
    Entitled,
    FreeWeekend,
}
