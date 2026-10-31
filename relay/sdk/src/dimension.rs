use std::sync::Arc;

use common::game_data::Dimension;

// The dimension parse, exposed so a bridge can name its own levels in the form frames carry.
//
// A bridge receives a modded dimension as `custom:<hex>` and must find the level it names.
// Hashing here keeps the formula in one place: a Kotlin copy of it could drift, and a frame
// whose hash no level matches is silent rather than an error.
#[derive(uniffi::Object)]
pub struct BvcDimensionCodec;

#[uniffi::export]
impl BvcDimensionCodec {
    #[uniffi::constructor]
    pub fn new() -> Arc<Self> {
        Arc::new(Self)
    }

    // The wire form of a raw dimension name: "nether", "custom:<hex>", …
    pub fn wire_form(&self, raw: String) -> String {
        Dimension::parse(&raw).wire_form().into_owned()
    }
}
