mod palette;
mod server;

pub use palette::MarkPalette;
pub use server::ServerGlyph;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A target's derived identity, so a controller can tell one world from another on a button.
///
/// Field names are snake_case, as the rest of this protocol is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Glyph {
    /// The column colour, `#rrggbb`.
    pub hue: String,
    /// Which of the mark's columns, so a caller can reference the same identity elsewhere.
    pub hue_index: u8,
    /// The 5x5 grid as seven lowercase hex characters. Bit `row * 5 + col`, set means filled.
    ///
    /// The whole grid rather than the fifteen independent bits, so a consumer never has to know
    /// the pattern is mirrored about its centre column.
    pub pattern: String,
}
