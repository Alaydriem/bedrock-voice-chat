use serde::Deserialize;

/// Profile setting key-value pair
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Setting {
    pub id: String,
    pub value: String,
}
