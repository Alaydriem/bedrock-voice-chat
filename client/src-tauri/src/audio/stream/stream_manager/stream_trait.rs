pub(crate) trait StreamTrait {
    fn stop(&mut self);
    fn is_stopped(&mut self) -> bool;
    fn start(&mut self) -> Result<(), anyhow::Error>;
}
