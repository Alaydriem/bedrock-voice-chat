use serde::Serialize;

/// The periodic liveness sample. `Server::Started` fires once per process, so a
/// server up since June is otherwise indistinguishable from one that never
/// started; this is the signal that separates them.
#[derive(Debug, Clone, Serialize)]
pub struct HeartbeatSnapshot {
    pub uptime_secs: u64,
    // The interval this sample covers. A starved heartbeat task skips ticks, so
    // without this a 30-minute sample is indistinguishable from a 15-minute one and
    // downstream cannot normalize the reach figures below.
    pub window_secs: u64,
    pub player_count: i64,
    pub peak_player_count: i64,
    // Distinct players, so these do NOT sum across heartbeats. Two people talking
    // all evening report 2 in each of 32 samples; summing yields 64 and means
    // nothing. Average or take a max over a range, never SUM.
    pub players_reached: u64,
    pub players_reached_proximity: u64,
    pub players_reached_channel: u64,
    pub players_reached_mutual: u64,
    pub players_reached_mutual_proximity: u64,
    pub players_reached_mutual_channel: u64,
    pub features_enabled: Vec<String>,
    // What the operator permits. Constant for the process.
    pub recording_enabled: bool,
    // Whether anyone is recording at the instant this sample was taken. Independent of
    // the capability above: a server that permits recording usually reports false here.
    pub recording_active: bool,
}
