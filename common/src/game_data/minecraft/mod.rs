mod hash;

use std::borrow::Cow;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub use hash::DimensionHash;

/// Minecraft world dimensions.
///
/// JSON carries a bare string (`overworld`, `nether`, `custom:<hex>`, …). Postcard carries
/// the variant index, plus the hash for `Other`, so the variant order is the wire contract:
/// append, never reorder.
//
// `remote = "Self"` makes the derive emit inherent `serialize`/`deserialize` functions
// instead of the traits; the trait impls below use them for the binary form.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Default, Serialize, Deserialize)]
#[serde(remote = "Self")]
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

    /// The string form JSON and the relay frame carry: a vanilla name (`overworld`,
    /// `nether`, `the_end`, `death`, `the_sift`) or `custom:<hex>`. `From<&str>` reads it
    /// back to the same value.
    pub fn as_api_str(&self) -> Cow<'static, str> {
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

// The one parse every producer path shares. Never returns `Overworld` for a name it does
// not know: an unknown dimension is isolated, not merged into another.
impl From<&str> for Dimension {
    fn from(raw: &str) -> Self {
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
            _ => Self::Other(DimensionHash::of(canonical)),
        }
    }
}

impl Serialize for Dimension {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if serializer.is_human_readable() {
            return serializer.serialize_str(&self.as_api_str());
        }

        Self::serialize(self, serializer)
    }
}

impl<'de> Deserialize<'de> for Dimension {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        if deserializer.is_human_readable() {
            let raw = Cow::<'de, str>::deserialize(deserializer)?;
            return Ok(Self::from(raw.as_ref()));
        }

        Self::deserialize(deserializer)
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
