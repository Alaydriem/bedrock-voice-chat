use std::time::Duration;

/// How often a player's `PlayerEnum` is put on the wire, on every leg that carries it.
///
/// One place, because the legs are a chain and only the slowest one decides what a listener
/// actually knows. A client that publishes its position slower than the server re-attaches it
/// makes the server spend wire weight re-sending a value that has not changed; a server that
/// re-attaches slower than the client publishes throws positions away before any listener
/// sees them. Neither shows up as a failure — spatial audio stays audible and merely lags the
/// speaker, which reads as the game being wrong rather than this number.
pub struct PositionCadence;

impl PositionCadence {
    /// Publications per second.
    ///
    /// Six, because a listener reconstructs a speaker from the last position it received and
    /// this bounds how far that reconstruction can trail the speaker's real one. At a sprint
    /// of roughly 5.6 blocks a second that is under a block of error, which is inside the
    /// distance over which the attenuation curve changes audibly.
    pub const PER_SECOND: u32 = 6;

    /// The gap between publications.
    ///
    /// Derived rather than written out, so the rate above is the only thing to change.
    pub const INTERVAL: Duration = Duration::from_millis(1000 / Self::PER_SECOND as u64);
}
