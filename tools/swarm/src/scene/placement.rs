/// Where one staged player stands, in the terms the feed reports rather than in
/// coordinates.
///
/// A screenshot is composed by bearing and distance — who sits where on the ring, how
/// far the label reads — so those are the inputs. Coordinates are derived, and only
/// because the position route takes nothing else.
#[derive(Debug, Clone)]
pub struct Placement {
    pub name: String,
    pub bearing_deg: f32,
    pub distance: f32,
    pub elevation: f32,
}

impl Placement {
    // Successive bearings this far apart never revisit a sector, so no two players
    // land on top of each other however many are staged. Even divisors of 360 do the
    // opposite: 12 players 30 degrees apart look like a clock face, which reads as
    // generated rather than as people standing around.
    const GOLDEN_ANGLE_DEG: f32 = 137.508;

    pub fn new(name: String, bearing_deg: f32, distance: f32, elevation: f32) -> Self {
        Self {
            name,
            bearing_deg: bearing_deg.rem_euclid(360.0),
            distance,
            elevation,
        }
    }

    /// Spreads `names` between `near` and `far` blocks on a non-repeating spiral.
    ///
    /// Deterministic in the order of `names`, so the same scene file produces the same
    /// picture on every platform — which is the only way four sets of screenshots read
    /// as one product photographed four times.
    pub fn ring(names: &[String], near: f32, far: f32) -> Vec<Self> {
        let last = names.len().saturating_sub(1);
        names
            .iter()
            .enumerate()
            .map(|(i, name)| {
                let step = if last == 0 {
                    0.0
                } else {
                    i as f32 / last as f32
                };
                Self::new(
                    name.clone(),
                    i as f32 * Self::GOLDEN_ANGLE_DEG,
                    near + (far - near) * step,
                    Self::elevation_for(i),
                )
            })
            .collect()
    }

    /// Coordinates for an observer at the origin facing yaw 0.
    ///
    /// The server reads bearing as `atan2(-dx, dz)` less the observer's yaw, so this is
    /// that inverted. Staging the observer at yaw 0 is what lets a bearing in the scene
    /// file survive to the screen unchanged.
    pub fn coordinates(&self, origin_y: f32) -> (f32, f32, f32) {
        let radians = self.bearing_deg.to_radians();
        (
            -self.distance * radians.sin(),
            origin_y + self.elevation,
            self.distance * radians.cos(),
        )
    }

    // A flat roster never exercises the elevation indicator, and one that varies every
    // entry looks like noise. Two in each seven stand off the observer's plane.
    fn elevation_for(index: usize) -> f32 {
        match index % 7 {
            3 => 5.0,
            5 => -4.0,
            _ => 0.0,
        }
    }
}
