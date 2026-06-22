use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct RelayFeature {
    // Seconds between self-announce cycles (`!bvca` injection). Absent uses the
    // production default (60s). Lowered only for integration tests.
    #[serde(default)]
    pub announce_interval_secs: Option<u64>,

    // Seconds between offer / idle-sweep / reconnect-grace cycles. Absent uses the
    // production default (5s). Lowered only for integration tests.
    #[serde(default)]
    pub orchestration_interval_secs: Option<u64>,

    // Seconds a peer link may sit idle before the sweep closes it. Absent uses the
    // production default (300s). Lowered only for integration tests.
    #[serde(default)]
    pub idle_timeout_secs: Option<u64>,
}
