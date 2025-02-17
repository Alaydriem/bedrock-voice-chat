mod stream_trait;
pub(crate) use stream_trait::StreamTrait;

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) enum IpcMessage {
    Terminate,
}