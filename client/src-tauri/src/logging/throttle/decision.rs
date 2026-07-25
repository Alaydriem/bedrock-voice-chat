pub(crate) enum ThrottleDecision {
    Emit { suppressed: u32 },
    Suppress,
}
