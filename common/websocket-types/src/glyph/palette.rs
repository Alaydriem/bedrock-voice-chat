/// The mark's 23 columns, in order.
///
/// A second copy of the hex values in `client/src/radial/core/mark/MarkData.ts`, which is
/// canonical because the palette tokens are defined against it. Two copies of one list with no
/// runtime cross-check between them: `tests/glyph/server.rs` is what keeps them equal.
pub struct MarkPalette;

impl MarkPalette {
    pub const HUES: [&'static str; 23] = [
        "#8239d8", "#8238d8", "#8238d8", "#6a50e9", "#466cf3", "#3d93ed", "#28bae1", "#21d8d8",
        "#26ddcd", "#34d8a0", "#3bd869", "#6fd846", "#aee236", "#f8e433", "#f8e434", "#f9bf21",
        "#f99a23", "#f9871d", "#f67414", "#f65021", "#f0422b", "#f8352b", "#f63125",
    ];

    pub const COLS: usize = Self::HUES.len();

    /// The colour of a column, wrapping so any index is valid.
    pub fn hue_at(index: usize) -> &'static str {
        Self::HUES[index % Self::COLS]
    }
}
