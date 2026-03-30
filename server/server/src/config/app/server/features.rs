use serde::{Deserialize, Serialize};

fn default_false() -> bool {
    false
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Features {
    #[serde(default = "default_false")]
    pub code_login: bool,
    #[serde(default = "default_false")]
    pub openapi_docs: bool,
}

impl Default for Features {
    fn default() -> Self {
        Features {
            code_login: default_false(),
            openapi_docs: default_false(),
        }
    }
}
