mod setting;
mod user;

pub(crate) use setting::Setting;
pub(crate) use user::ProfileUser;

use serde::Deserialize;

/// Response from Xbox Live profile endpoint
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProfileResponse {
    pub profile_users: Vec<ProfileUser>,
}
