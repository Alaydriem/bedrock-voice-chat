/// Error type for timecode track operations
#[derive(Debug)]
pub enum TimecodeError {
    /// Missing required field in builder
    MissingField(&'static str),
}

impl std::fmt::Display for TimecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TimecodeError::MissingField(field) => {
                write!(f, "Missing required field: {}", field)
            }
        }
    }
}

impl std::error::Error for TimecodeError {}
