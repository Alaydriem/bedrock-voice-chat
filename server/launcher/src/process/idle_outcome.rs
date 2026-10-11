#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdleOutcome {
    Elapsed,
    Signalled,
    BdsExited(i32),
}
