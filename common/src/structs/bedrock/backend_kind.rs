use serde::{Deserialize, Serialize};
use ts_rs::TS;

// Which kind of upstream the user started: a direct Bedrock server proxy or
// a Realms connection. Drives small UX differences in the connection-info
// modal (label copy, what counts as "remote server").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub enum BedrockBackendKind {
    Direct,
    Realm,
}
