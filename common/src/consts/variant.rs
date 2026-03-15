use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Variant {
    #[serde(rename = "dev")]
    Dev,
    #[serde(rename = "release")]
    Release,
}

impl std::fmt::Display for Variant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Variant::Dev => write!(f, "development"),
            Variant::Release => write!(f, "production"),
        }
    }
}

pub fn get_variant() -> Variant {
    if cfg!(debug_assertions) {
        Variant::Dev
    } else {
        Variant::Release
    }
}