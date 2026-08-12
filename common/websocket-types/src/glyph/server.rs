use super::{Glyph, MarkPalette};

/// Derives a target's identity from its name.
///
/// The Rust counterpart of `client/src/radial/core/glyph/ServerGlyph.ts`. No upload, no avatar
/// service, no default grey box: a name produces a hue and a mirrored block pattern, so every
/// client that knows the name agrees on the tile without anything being transmitted.
pub struct ServerGlyph;

impl ServerGlyph {
    pub const GRID: usize = 5;

    pub fn of(name: &str) -> Glyph {
        let hash = Self::fnv1a(name);
        let hue_index = (hash as usize) % MarkPalette::COLS;

        let mut pattern: u32 = 0;
        for row in 0..Self::GRID {
            for col in 0..3 {
                if (hash >> (row * 3 + col)) & 1 == 0 {
                    continue;
                }
                pattern |= 1 << (row * Self::GRID + col);
                if col < 2 {
                    pattern |= 1 << (row * Self::GRID + (4 - col));
                }
            }
        }

        Glyph {
            hue: MarkPalette::hue_at(hue_index).to_string(),
            hue_index: hue_index as u8,
            pattern: format!("{pattern:07x}"),
        }
    }

    /// FNV-1a, 32-bit, over UTF-16 code units.
    ///
    /// Code units rather than bytes, because the TypeScript side hashes `charCodeAt` and the two
    /// disagree for every name outside ASCII. A world name with an accent would otherwise draw
    /// one tile on a controller and a different one on the desktop.
    fn fnv1a(input: &str) -> u32 {
        let mut hash: u32 = 2166136261;
        for unit in input.encode_utf16() {
            hash ^= unit as u32;
            hash = hash.wrapping_mul(16777619);
        }
        hash
    }
}
