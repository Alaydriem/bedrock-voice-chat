//! Xbox Live profile response DTOs

use serde::Deserialize;

/// Response from Xbox Live profile endpoint
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProfileResponse {
    pub profile_users: Vec<ProfileUser>,
}

/// Individual user profile
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProfileUser {
    pub settings: Vec<Setting>,
}

/// Profile setting key-value pair
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Setting {
    pub id: String,
    pub value: String,
}
