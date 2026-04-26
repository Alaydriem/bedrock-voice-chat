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

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct MinecraftJavaProfile {
    #[serde(rename = "id")]
    pub uuid: String,
    pub name: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn java_profile_deserializes_id_and_name() {
        let body = r#"{"id":"d8d5a92366af4b1d8c6d3e8b5a9b1234","name":"CoolBuilder42"}"#;
        let profile: MinecraftJavaProfile = serde_json::from_str(body).unwrap();
        assert_eq!(profile.uuid, "d8d5a92366af4b1d8c6d3e8b5a9b1234");
        assert_eq!(profile.name, "CoolBuilder42");
    }

    #[test]
    fn java_profile_ignores_extra_fields() {
        let body = r#"{"id":"abc","name":"x","skins":[],"capes":[]}"#;
        let profile: MinecraftJavaProfile = serde_json::from_str(body).unwrap();
        assert_eq!(profile.uuid, "abc");
        assert_eq!(profile.name, "x");
    }
}
