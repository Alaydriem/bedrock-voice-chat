use common::Coordinate;

/// A square of the world, used to find an observer's neighbours without walking it.
///
/// The cell is the feed's scope, not voice range, so the eight cells around an observer's
/// own are guaranteed to contain everyone within scope of them. A smaller cell would leave
/// somebody two cells away who is nonetheless inside scope — invisible, intermittently,
/// depending only on where the grid lines happened to fall.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GridCell {
    pub x: i32,
    pub z: i32,
}

impl GridCell {
    pub fn of(position: &Coordinate, size: f32) -> Self {
        // A degenerate size would put the whole world in one cell, which is slow but
        // correct — unlike dividing by zero.
        let size = if size > 1.0 { size } else { 1.0 };
        Self {
            x: (position.x / size).floor() as i32,
            z: (position.z / size).floor() as i32,
        }
    }

    /// This cell and the eight around it.
    pub fn ring(&self) -> [GridCell; 9] {
        let mut cells = [*self; 9];
        let mut i = 0;
        for dx in -1..=1 {
            for dz in -1..=1 {
                cells[i] = GridCell {
                    x: self.x + dx,
                    z: self.z + dz,
                };
                i += 1;
            }
        }
        cells
    }
}
