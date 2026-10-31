mod hash;
mod name_log;
mod wire;

use std::borrow::Cow;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub use hash::DimensionHash;
pub use name_log::DimensionNameLog;
use wire::DimensionWire;

/// Minecraft world dimensions.
///
/// JSON carries a bare string (`overworld`, `nether`, `custom:<hex>`, …). Postcard carries
/// the variant index, plus the hash for `Other`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Default)]
pub enum Dimension {
    #[default]
    Overworld,
    TheEnd,
    TheNether,
    Death,
    // Reserved for the vanilla dimension due in 2027. Nothing parses to it until its id is
    // published; until then that id arrives as `Other` and is still gated on its own.
    TheSift,
    // Any dimension without a variant: modded, add-on, or a renamed Paper world.
    Other(DimensionHash),
}

impl Dimension {
    const MINECRAFT_PREFIX: &'static str = "minecraft:";
    const CUSTOM_PREFIX: &'static str = "custom:";

    /// The one parse every producer path shares. Never returns `Overworld` for a name it
    /// does not know: an unknown dimension is isolated, not merged into another.
    pub fn parse(raw: &str) -> Self {
        if let Some(hash) = raw
            .strip_prefix(Self::CUSTOM_PREFIX)
            .and_then(DimensionHash::from_hex)
        {
            return Self::Other(hash);
        }

        let canonical = raw.strip_prefix(Self::MINECRAFT_PREFIX).unwrap_or(raw);
        match canonical {
            "overworld" => Self::Overworld,
            "nether" | "the_nether" => Self::TheNether,
            "the_end" => Self::TheEnd,
            "death" => Self::Death,
            _ => {
                let hash = DimensionHash::of(canonical);
                DimensionNameLog::observe(raw, hash);
                Self::Other(hash)
            }
        }
    }

    /// The JSON string form, which `parse` reads back to the same value.
    pub fn wire_form(&self) -> Cow<'static, str> {
        match self {
            Self::Overworld => Cow::Borrowed("overworld"),
            Self::TheNether => Cow::Borrowed("nether"),
            Self::TheEnd => Cow::Borrowed("the_end"),
            Self::Death => Cow::Borrowed("death"),
            Self::TheSift => Cow::Borrowed("the_sift"),
            Self::Other(hash) => Cow::Owned(format!("{}{}", Self::CUSTOM_PREFIX, hash.to_hex())),
        }
    }
}

impl From<&str> for Dimension {
    fn from(value: &str) -> Self {
        Self::parse(value)
    }
}

impl Serialize for Dimension {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if serializer.is_human_readable() {
            return serializer.serialize_str(&self.wire_form());
        }

        DimensionWire::from(*self).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Dimension {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        if deserializer.is_human_readable() {
            let raw = Cow::<'de, str>::deserialize(deserializer)?;
            return Ok(Self::parse(&raw));
        }

        DimensionWire::deserialize(deserializer).map(Self::from)
    }
}

// Described as a string, matching the human-readable serde form, so the generated OpenAPI
// document agrees with what an HTTP client sends.
#[cfg(feature = "openapi")]
impl schemars::JsonSchema for Dimension {
    fn schema_name() -> String {
        "Dimension".to_string()
    }

    fn json_schema(generator: &mut schemars::r#gen::SchemaGenerator) -> schemars::schema::Schema {
        <String as schemars::JsonSchema>::json_schema(generator)
    }
}
