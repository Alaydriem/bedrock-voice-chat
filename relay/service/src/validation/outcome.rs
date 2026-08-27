#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationOutcome {
    Passed,
    Failed { consecutive: i32 },
    Suspended,
}
