use common::structs::relay::enroll::PlayPreflightOutcome;

/// Keeps a preflight that fails the same way every few seconds from filling the log.
pub struct PreflightLogGate {
    last: Option<&'static str>,
}

impl PreflightLogGate {
    pub fn new() -> Self {
        Self { last: None }
    }

    /// True the first time an outcome kind is seen, and again after a different kind.
    ///
    /// `RateLimited` is never logged here and does not count as a kind: the server waits and
    /// asks again without the operator, and the failure on either side of it is the same
    /// failure.
    pub fn should_log(&mut self, outcome: &PlayPreflightOutcome) -> bool {
        if matches!(outcome, PlayPreflightOutcome::RateLimited { .. }) {
            return false;
        }
        let kind = outcome.kind();
        if self.last == Some(kind) {
            return false;
        }
        self.last = Some(kind);
        true
    }
}

impl Default for PreflightLogGate {
    fn default() -> Self {
        Self::new()
    }
}
