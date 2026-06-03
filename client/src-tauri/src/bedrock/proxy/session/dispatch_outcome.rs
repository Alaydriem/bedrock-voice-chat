pub enum DispatchOutcome {
    Continue,
    SessionEnded {
        reason: &'static str,
        detail: Option<String>,
    },
}
