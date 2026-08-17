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

impl Dimension {
    // The inverse of `From<&str>`, in the bare serde form. Named for the API rather
    // than as a `Display` impl because the Bedrock id form carries a `minecraft:`
    // prefix this deliberately omits, and the two are not interchangeable at a call
    // site that feeds either back in.
    pub fn as_api_str(&self) -> &'static str {
        match self {
            Dimension::Overworld => "overworld",
            Dimension::TheNether => "nether",
            Dimension::TheEnd => "the_end",
            Dimension::Death => "death",
        }
    }
}
