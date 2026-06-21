use tokio::task::JoinHandle;

// Driver returned by a source after it begins producing frames: the tokio handle
// the sender task is paired against, plus the oneshot the listener holds to stop
// a live cpal stream (None for sources that stop on their own feed closing).
pub(crate) struct SourceDriver {
    pub handle: JoinHandle<()>,
    pub shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
}
