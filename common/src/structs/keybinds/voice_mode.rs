use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub enum VoiceMode {
    #[default]
    OpenMic,
    PushToTalk,
}
