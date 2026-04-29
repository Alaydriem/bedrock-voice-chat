use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum PermissionEffect {
    Allow,
    Deny,
}

impl PermissionEffect {
    pub fn from_db(effect: i32) -> Self {
        if effect & 1 == 1 {
            PermissionEffect::Allow
        } else {
            PermissionEffect::Deny
        }
    }

    pub fn to_db(&self) -> i32 {
        match self {
            PermissionEffect::Allow => 1,
            PermissionEffect::Deny => 0,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            PermissionEffect::Allow => "allow",
            PermissionEffect::Deny => "deny",
        }
    }
}

impl std::fmt::Display for PermissionEffect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
