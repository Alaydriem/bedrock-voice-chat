/// Internal control-plane state-change signal. This is deliberately NOT the
/// Stream-Deck WebSocket `StateData` contract — that is an external `JsonSchema`
/// wire format and must not grow fields for internal plumbing. Producers fire a
/// signal when local audio state changes; the `QueryStateReporter` coalesces
/// them into ServerBound `QueryState` / `PlayerPreference` reports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlStateSignal {
    // Self mute/deafen/record changed, or a fresh connection needs a snapshot.
    SelfState,
    // The persisted per-player gain/mute preferences changed.
    Preferences,
    // The in-game panel requested a fresh snapshot (bvc:ctl:sync) scoped to the
    // players it is showing; rides the !bvcs: reverse path only.
    Sync { targets: Vec<String> },
}
