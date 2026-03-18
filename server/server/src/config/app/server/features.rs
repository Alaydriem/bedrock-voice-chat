use serde::{Deserialize, Serialize};

fn default_code_login() -> bool {
    false
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Features {
    #[serde(default = "default_code_login")]
    pub code_login: bool,
}

impl Default for Features {
    fn default() -> Self {
        Features {
            code_login: default_code_login(),
        }
    }
}
