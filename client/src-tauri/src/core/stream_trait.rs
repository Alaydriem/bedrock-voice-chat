/// The StreamTrait interface defines the methods that must be implemented by
/// Any streaming server, at minimum.
/// stop, is_stopped, start, and metadata.
/// This should be implemented both by network and audio streams.
pub(crate) trait StreamTrait {
    async fn stop(&mut self) -> Result<(), anyhow::Error>;
    fn is_stopped(&mut self) -> bool;
    async fn start(&mut self) -> Result<(), anyhow::Error>;
    async fn metadata(&mut self, key: String, value: String) -> Result<(), anyhow::Error>;
}
