/// What a transport failure means for the endpoint it happened on.
///
/// The two facts are independent. A misconfigured client fails against the same
/// dead host for as long as it runs, so the failure worth an error record is the
/// first one after the endpoint was last reachable; the rest are the same outage
/// still going. Opening is a separate event that can happen several times inside
/// one outage, once per cooldown the half-open probe fails.
pub struct FailureReport {
    /// First transport failure since this endpoint was last reachable.
    pub first: bool,
    /// This failure opened the breaker; further requests short-circuit until the
    /// cooldown elapses.
    pub opened: bool,
}
