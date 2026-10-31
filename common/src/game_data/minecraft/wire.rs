use serde::{Deserialize, Serialize};

use super::{Dimension, DimensionHash};

// The binary form of `Dimension`. Postcard encodes the variant index, so the order here
// is the wire contract: appended, never reordered.
#[derive(Serialize, Deserialize)]
pub(crate) enum DimensionWire {
    Overworld,
    TheEnd,
    TheNether,
    Death,
    TheSift,
    Other(DimensionHash),
}

impl From<Dimension> for DimensionWire {
    fn from(dimension: Dimension) -> Self {
        match dimension {
            Dimension::Overworld => Self::Overworld,
            Dimension::TheEnd => Self::TheEnd,
            Dimension::TheNether => Self::TheNether,
            Dimension::Death => Self::Death,
            Dimension::TheSift => Self::TheSift,
            Dimension::Other(hash) => Self::Other(hash),
        }
    }
}

impl From<DimensionWire> for Dimension {
    fn from(wire: DimensionWire) -> Self {
        match wire {
            DimensionWire::Overworld => Self::Overworld,
            DimensionWire::TheEnd => Self::TheEnd,
            DimensionWire::TheNether => Self::TheNether,
            DimensionWire::Death => Self::Death,
            DimensionWire::TheSift => Self::TheSift,
            DimensionWire::Other(hash) => Self::Other(hash),
        }
    }
}
