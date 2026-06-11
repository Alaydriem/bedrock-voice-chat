use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct RealmEntry {
    pub id: u64,
    pub name: String,
    pub motd: String,
    pub state: String,
    pub owner_uuid: String,
}
