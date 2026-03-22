use serde::{Deserialize, Serialize};

fn default_file_path() -> String {
    "./assets/audio".to_string()
}

fn default_max_concurrent_per_uuid() -> usize {
    5
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Audio {
    #[serde(default = "default_file_path")]
    pub file_path: String,

    #[serde(default = "default_max_concurrent_per_uuid")]
    pub max_concurrent_per_uuid: usize,
}

impl Default for Audio {
    fn default() -> Self {
        Self {
            file_path: default_file_path(),
            max_concurrent_per_uuid: default_max_concurrent_per_uuid(),
        }
    }
}
