use serde::{Deserialize, Serialize};

/// Minecraft-specific structs

/// Minecraft world dimensions
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, Default)]
pub enum Dimension {
    #[default]
    #[serde(rename = "overworld")]
    Overworld,
    #[serde(rename = "the_end")]
    TheEnd,
    #[serde(rename = "nether")]
    TheNether,
    #[serde(rename = "death")]
    Death,
}
