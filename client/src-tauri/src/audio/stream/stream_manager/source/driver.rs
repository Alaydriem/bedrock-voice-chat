// Driver returned by a source after it begins producing frames.
pub(crate) struct SourceDriver {
    // The live cpal stream, for the source that has one.
    //
    // Held rather than parked on a thread that blocks until told to quit: dropping it pauses the
    // device and hands it back, so the release is ordered with the next open instead of racing it
    // across a channel and a fixed sleep. `None` for a source whose feed stops on its own, and for
    // a cpal stream that failed to build.
    pub stream: Option<rodio::cpal::Stream>,
}
