use std::collections::HashMap;
use serde::{Deserialize, Serialize};

fn default_permissions() -> HashMap<String, i32> {
    let mut map = HashMap::new();
    map.insert("audio_upload".to_string(), 1);
    map.insert("audio_delete".to_string(), 1);
    map
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Permissions {
    #[serde(default = "default_permissions")]
    pub defaults: HashMap<String, i32>,
}

impl Default for Permissions {
    fn default() -> Self {
        Self {
            defaults: default_permissions(),
        }
    }
}
