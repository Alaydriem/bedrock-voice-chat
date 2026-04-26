use serde::{Deserialize, Serialize};

/// Common result from authentication - the authenticated user's identity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResult {
    /// The user's display name / gamertag
    pub gamertag: String,
    /// The user's profile picture URL (base64 encoded)
    pub gamerpic: String,
    /// The user's Minecraft Java username (if they own Java Edition)
    #[serde(default)]
    pub minecraft_username: Option<String>,
    #[serde(default)]
    pub minecraft_uuid: Option<String>,
}

impl AuthResult {
    /// Create a new AuthResult
    pub fn new(gamertag: String, gamerpic: String) -> Self {
        Self {
            gamertag,
            gamerpic,
            minecraft_username: None,
            minecraft_uuid: None,
        }
    }

    /// Create an AuthResult with no profile picture
    pub fn without_gamerpic(gamertag: String) -> Self {
        Self {
            gamertag,
            gamerpic: String::new(),
            minecraft_username: None,
            minecraft_uuid: None,
        }
    }

    /// Set the Minecraft Java username
    pub fn with_minecraft_username(mut self, username: Option<String>) -> Self {
        self.minecraft_username = username;
        self
    }

    pub fn with_java_profile(
        mut self,
        username: Option<String>,
        uuid: Option<String>,
    ) -> Self {
        self.minecraft_username = username;
        self.minecraft_uuid = uuid;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn with_java_profile_sets_both_fields() {
        let r = AuthResult::new("Tag".into(), "pic".into())
            .with_java_profile(Some("JavaName".into()), Some("uuid-1".into()));
        assert_eq!(r.minecraft_username.as_deref(), Some("JavaName"));
        assert_eq!(r.minecraft_uuid.as_deref(), Some("uuid-1"));
    }

    #[test]
    fn with_java_profile_accepts_none() {
        let r = AuthResult::new("Tag".into(), "pic".into())
            .with_java_profile(None, None);
        assert!(r.minecraft_username.is_none());
        assert!(r.minecraft_uuid.is_none());
    }

    #[test]
    fn auth_result_serializes_uuid_when_present() {
        let r = AuthResult::new("Tag".into(), "pic".into())
            .with_java_profile(Some("J".into()), Some("u".into()));
        let json = serde_json::to_string(&r).unwrap();
        assert!(json.contains(r#""minecraft_uuid":"u""#));
    }
}
