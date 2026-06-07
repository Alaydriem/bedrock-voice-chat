use serde::{Deserialize, Serialize};

/// Minecraft-specific structs

/// Minecraft world dimensions
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, Hash, Default)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
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

impl From<&str> for Dimension {
    // Accepts both the Bedrock id form the mod/FE emits ("minecraft:overworld")
    // and the bare serde form ("overworld"); the suffixes match the serde renames.
    fn from(value: &str) -> Self {
        match value.strip_prefix("minecraft:").unwrap_or(value) {
            "overworld" => Dimension::Overworld,
            "nether" => Dimension::TheNether,
            "the_end" => Dimension::TheEnd,
            "death" => Dimension::Death,
            _ => Dimension::Overworld,
        }
    }
}
