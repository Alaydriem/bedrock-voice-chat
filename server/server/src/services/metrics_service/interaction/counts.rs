/// Distinct-participant figures for one route over one window. `reached` counts
/// delivery, not audition: a client-side mute is invisible to the server, and the
/// delivery path never consults a recipient's deafen state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct InteractionCounts {
    pub reached: u64,
    pub mutual: u64,
}
