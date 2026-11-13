// What a probe of a candidate address found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeOutcome {
    Answered,
    TimedOut,
    Refused,
    WrongService,
}
