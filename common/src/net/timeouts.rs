use std::time::Duration;

/// Every budget the connect path and the reachability probes are allowed to spend.
///
/// One place, because these numbers only make sense relative to each other. A probe that
/// gave up sooner than the connect reports no voice path on a server the connect then
/// reaches; a connect that gave up sooner than a probe contradicts a verdict the screen has
/// already shown. Both failures are invisible on a fast network and reproduce only where the
/// latency is, which is the far side of the planet.
pub struct NetTimeouts;

impl NetTimeouts {
    /// One QUIC handshake attempt, on the connect path and in the probe alike.
    ///
    /// Sized for the worst path a player actually has rather than a good one: an
    /// intercontinental round trip is around 300ms, a handshake costs one of them, and a
    /// lost Initial costs another plus PTO backoff. Anything tight enough to look generous
    /// against a local server abandons a distant one that was about to succeed, which
    /// arrives as "the server is down".
    pub const HANDSHAKE: Duration = Duration::from_secs(7);

    /// The version-negotiation probe: one UDP round trip, no TLS, no certificate.
    ///
    /// Deliberately far shorter than a handshake. It runs on the connect path where the
    /// delay is visible, it retransmits once inside its own budget, and a distant server
    /// that outlasts it is not lost — the orchestrator escalates to the handshake probe,
    /// which has the full budget.
    pub const NEGOTIATION: Duration = Duration::from_millis(750);

    /// One HTTPS request, for the probe that establishes a server answers at all.
    ///
    /// Matches the handshake: this is the request that decides whether a failure is the
    /// server being absent or the credentials being refused, and answering "absent" about a
    /// server that was merely slow sends somebody to ask their operator about nothing.
    pub const HTTPS: Duration = Duration::from_secs(7);
}
