// The boundaries behind a status warning. They live beside `LinkQuality` so a panel, a log
// line, and a rollup cannot disagree about what "degraded" means.
//
// These values are provisional. Real boundaries are an operator judgement that needs field
// data the rollup does not yet provide, so they are named and exported rather than inlined:
// changing one is a one-line edit, and every test references the constant instead of the
// number.

pub const LOSS_DEGRADED_PCT: f32 = 1.0;
pub const LOSS_BAD_PCT: f32 = 3.0;
pub const RTT_DEGRADED_MS: u32 = 200;
pub const RTT_BAD_MS: u32 = 400;
