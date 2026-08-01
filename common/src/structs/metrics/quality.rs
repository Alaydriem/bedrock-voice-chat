use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::thresholds::{LOSS_BAD_PCT, LOSS_DEGRADED_PCT, RTT_BAD_MS, RTT_DEGRADED_MS};

// Variants ascend in severity so `Ord` makes the worst dimension win.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub enum LinkQuality {
    Good,
    Degraded,
    Bad,
}

impl LinkQuality {
    // Loss and round trip are assessed separately and the worse verdict is returned. A link
    // that is lossless but 500 ms away is not healthy, and neither is a 20 ms link dropping
    // one packet in twenty.
    pub fn classify(loss_pct: f32, rtt_ms: u32) -> Self {
        let by_loss = if loss_pct >= LOSS_BAD_PCT {
            Self::Bad
        } else if loss_pct >= LOSS_DEGRADED_PCT {
            Self::Degraded
        } else {
            Self::Good
        };

        let by_rtt = if rtt_ms >= RTT_BAD_MS {
            Self::Bad
        } else if rtt_ms >= RTT_DEGRADED_MS {
            Self::Degraded
        } else {
            Self::Good
        };

        by_loss.max(by_rtt)
    }
}
