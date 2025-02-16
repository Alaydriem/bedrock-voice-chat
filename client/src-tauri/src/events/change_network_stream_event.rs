use serde::{ Serialize, Deserialize };

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ChangeNetworkStreamEvent {
  pub server: String
}