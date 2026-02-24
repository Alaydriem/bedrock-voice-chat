use std::collections::HashMap;

use serde::{Deserialize, Serialize};

fn default_permissions() -> HashMap<String, i32> {
    let mut map = HashMap::new();
    map.insert("audio_upload".to_string(), 1);
    map.insert("audio_delete".to_string(), 1);
    map
}

/// Permission defaults configuration.
/// Maps permission names (serde-serialized `Permission` enum values) to effect integers.
/// 0 = deny, 1 = allow (bitwise-extensible).
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApplicationConfigPermissions {
    #[serde(flatten, default = "default_permissions")]
    pub defaults: HashMap<String, i32>,
}

impl Default for ApplicationConfigPermissions {
    fn default() -> Self {
        Self {
            defaults: default_permissions(),
        }
    }
}
