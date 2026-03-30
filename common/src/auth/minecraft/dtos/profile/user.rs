use serde::Deserialize;

use super::Setting;

/// Individual user profile
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProfileUser {
    pub settings: Vec<Setting>,
}
