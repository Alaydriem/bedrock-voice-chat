// Which playback route a set of counters belongs to. A speaker can be heard on both at once —
// positionally and flat — and each route has its own jitter buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PeerRoute {
    Normal,
    Spatial,
}
