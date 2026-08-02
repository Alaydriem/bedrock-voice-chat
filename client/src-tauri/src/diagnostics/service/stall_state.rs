// How many consecutive ticks have observed no inbound QUIC packets. Held across ticks so one quiet
// acknowledgement window cannot trip the stall flag on its own.
#[derive(Debug, Default)]
pub(super) struct StallState {
    pub(super) consecutive: u32,
}
