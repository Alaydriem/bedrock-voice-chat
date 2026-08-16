/// What the watchdog concluded from one reading of the capture counter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureVerdict {
    /// Frames advanced, or capture is not expected at the moment.
    Healthy,
    /// Capture is expected and nothing arrived, but not for long enough to act on.
    Quiet,
    /// Capture has been expected and absent for the whole threshold. Rebuild the stream.
    Dead,
}
