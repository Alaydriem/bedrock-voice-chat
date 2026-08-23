
// What a child dispatcher decided: whether the session continues, and whether
// the session state moved. The caller needs both — it owns the cache write a
// state change implies, and only it can end the session.
//
// Generic over both payloads so a richer state-delta can replace the flag
// without a second container type.
pub struct DispatchResult<O, S> {
    pub outcome: O,
    pub state_changed: S,
}
