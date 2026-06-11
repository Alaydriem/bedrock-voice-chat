use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct NetworkInterface {
    pub name: String,
    pub ip: String,
    pub is_ipv4: bool,
}
