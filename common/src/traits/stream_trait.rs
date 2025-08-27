/// The StreamTrait interface defines the methods that must be implemented by
/// any streaming component at minimum: stop, is_stopped, start, and metadata.
/// This should be implemented both by network and audio streams.
#[allow(async_fn_in_trait)]
pub trait StreamTrait {
    /// Stop the stream gracefully
    async fn stop(&mut self) -> Result<(), anyhow::Error>;

    /// Check if the stream is stopped
    fn is_stopped(&self) -> bool;

    /// Start the stream
    async fn start(&mut self) -> Result<(), anyhow::Error>;

    /// Set metadata for the stream
    async fn metadata(&mut self, key: String, value: String) -> Result<(), anyhow::Error>;
}
