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

    /// How long QUIC runs alone before the WebSocket alternative is also dialled.
    ///
    /// A head start rather than a dead heat: a WebSocket attempt that wins costs the
    /// server a handshake and a registered session for a link nobody uses. Where QUIC
    /// works, this expires after QUIC has already won and the alternative never opens.
    pub const WEBSOCKET_HEAD_START: Duration = Duration::from_secs(1);

    /// How long a QUIC attempt may keep running once WebSocket is connected and waiting.
    ///
    /// Matched to `HANDSHAKE`: anything shorter hands the session to WebSocket while a
    /// distant QUIC handshake is still inside the budget this file grants it.
    pub const QUIC_OVERTAKE: Duration = Self::HANDSHAKE;

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
