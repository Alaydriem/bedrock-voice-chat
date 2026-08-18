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

    /// How far the fallback has to beat QUIC before it is preferred over it.
    ///
    /// A margin rather than a comparison, because the faster answer is not always the
    /// better path. The fallback costs a TCP handshake and a TLS handshake where QUIC
    /// costs one round trip, so distance inflates the fallback more than it inflates
    /// QUIC — a player who is merely far away can never open a gap this wide, and keeps
    /// QUIC. A gap this wide means QUIC is losing Initials and backing off, which is a
    /// degraded path rather than a distant one, and voice carried over it is worse than
    /// voice carried over TCP.
    pub const WEBSOCKET_PREFERENCE_MARGIN: Duration = Duration::from_secs(2);

    /// How long the QUIC legs of a probe may keep running after one of them has answered.
    ///
    /// The answer that matters is already in hand at this point: an endpoint that answers
    /// later sorts below one that answered sooner, and one that never answers sorts last.
    /// The grace exists only so endpoints that are genuinely close together are all
    /// measured, rather than the first to land deciding the address family alone.
    pub const PROBE_SETTLE: Duration = Duration::from_millis(250);

    /// One HTTPS request, for the probe that establishes a server answers at all.
    ///
    /// Matches the handshake: this is the request that decides whether a failure is the
    /// server being absent or the credentials being refused, and answering "absent" about a
    /// server that was merely slow sends somebody to ask their operator about nothing.
    pub const HTTPS: Duration = Duration::from_secs(7);

    /// One TLS handshake against the WebSocket voice transport, for the probe that decides
    /// whether a UDP-blocked client has a path at all.
    ///
    /// Matched to `HTTPS` because it is the same TCP path to the same port. Anything shorter
    /// would report no fallback on a server the HTTPS probe reached in the same run, and the
    /// two answers would contradict each other on exactly the slow networks this transport
    /// exists for.
    pub const VOICE_WEBSOCKET: Duration = Self::HTTPS;
}
