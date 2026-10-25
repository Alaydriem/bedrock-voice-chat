#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationOutcome {
    Passed,
    Recovered,
    Failed { consecutive: i32 },
    Revoked,
}
