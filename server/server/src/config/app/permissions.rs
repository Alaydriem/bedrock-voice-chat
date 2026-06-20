use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
