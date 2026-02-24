/// Runtime state for the server
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeState {
    /// Server is not started
    Stopped,
    /// Server is starting up
    Starting,
    /// Server is running
    Running,
    /// Server is shutting down
    ShuttingDown,
}
