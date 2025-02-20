pub(crate) trait StreamTrait {
    async fn stop(&mut self) -> Result<(), anyhow::Error>;
    fn is_stopped(&mut self) -> bool;
    async fn start(&mut self) -> Result<(), anyhow::Error>;
    fn metadata(&mut self, key: String, value: String) -> Result<(), anyhow::Error>;
}
