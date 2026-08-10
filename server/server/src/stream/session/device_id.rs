use common::structs::metrics::TransportKind;
use std::sync::atomic::{AtomicU64, Ordering};

/// Mints device ids for WebSocket sessions, disjoint from every QUIC one.
///
/// A device id keys `ConnectionRegistry`, so the two transports must never mint the same
/// value. s2n-quic's `InternalConnectionId` is a `u64` counter starting at zero and
/// incrementing by one — its own comment notes that exhausting it takes years at a billion
/// ids per second, so the high bit is unreachable there. Setting it here makes the two
/// spaces provably disjoint rather than merely unlikely to collide, which a second counter
/// starting at zero would not be: that one would collide on the first session of each
/// transport.
///
/// Every consumer treats the value as opaque — a map key, an `is_some()` check, a log
/// field — so nothing depends on the ids being dense or ordered. A useful side effect is
/// that a log line's device id now says which transport carried the session.
pub(crate) struct WebSocketDeviceId {
    next: AtomicU64,
}

impl WebSocketDeviceId {
    const WEBSOCKET_MARKER: u64 = 1 << 63;

    pub(crate) fn new() -> Self {
        Self {
            next: AtomicU64::new(0),
        }
    }

    pub(crate) fn next(&self) -> u64 {
        Self::WEBSOCKET_MARKER | self.next.fetch_add(1, Ordering::Relaxed)
    }

    /// Which transport minted a device id.
    ///
    /// The registry keys on the device id and never learns the transport any other way,
    /// so reading it back out of the id is what lets a metric carry the dimension without
    /// threading a second field through every call.
    pub(crate) fn transport_of(device: u64) -> TransportKind {
        if device & Self::WEBSOCKET_MARKER == 0 {
            TransportKind::Quic
        } else {
            TransportKind::WebSocket
        }
    }
}

impl Default for WebSocketDeviceId {
    fn default() -> Self {
        Self::new()
    }
}
