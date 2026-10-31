use serde::{Deserialize, Serialize};

use super::GroupLinkEvent;

/// The events after the sequence number the bridge last saw.
///
/// `reset` means events the bridge never saw were dropped, or the server restarted. The bridge
/// then resyncs from both sides instead of trusting the events alone.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
pub struct GroupLinkDrain {
    pub events: Vec<GroupLinkEvent>,
    pub reset: bool,
    /// Random per server start, and below 2^53 so every JSON reader keeps it exact. A bridge
    /// that sees it change knows the sequence numbers started over, and resyncs.
    pub epoch: u64,
}
