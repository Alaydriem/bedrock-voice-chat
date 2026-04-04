use std::collections::HashMap;
use serde::{Deserialize, Serialize};

fn default_permissions() -> HashMap<String, bool> {
    HashMap::new()
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Permissions {
    #[serde(default = "default_permissions")]
    pub defaults: HashMap<String, bool>,
}

impl Default for Permissions {
    fn default() -> Self {
        Self {
            defaults: default_permissions(),
        }
    }
}
