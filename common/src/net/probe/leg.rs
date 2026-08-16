use crate::structs::reachability::EndpointReachability;

/// Which leg of a reachability measurement a finished probe belongs to.
///
/// The three legs share one `JoinSet` so a completion is seen the moment it lands rather than
/// when its own leg is drained. That is what lets a measurement answer early: the WebSocket
/// probe finishes in milliseconds while the QUIC walk still has seconds of budget left, and
/// draining the sets in turn would hide that answer behind the slowest of them.
pub enum MeasuredLeg {
    Quic(EndpointReachability),
    Https(EndpointReachability),
    Ws(EndpointReachability),
}
