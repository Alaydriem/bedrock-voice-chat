use serde::Deserialize;

/// Xbox User Info
#[derive(Deserialize)]
pub(crate) struct Xui {
    #[serde(rename = "uhs")]
    pub user_hash: String,
}
