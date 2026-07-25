/// Outcome of a request routed through the endpoint breaker. `Open` means the
/// breaker short-circuited before touching the network; `Transport` carries a
/// genuine send failure that was already recorded against the breaker.
pub(crate) enum SendError {
    Open,
    Transport(common::reqwest::Error),
}
