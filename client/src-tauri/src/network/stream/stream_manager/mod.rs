mod input;
mod output;

pub(crate) use input::InputStream;
pub(crate) use output::OutputStream;
pub(crate) use crate::core::StreamTrait;

pub(crate) enum StreamTraitType {
    Input(InputStream),
    Output(OutputStream),
}

impl StreamTrait for StreamTraitType {
    fn is_stopped(&mut self) -> bool {
        match self {
            Self::Input(stream) => stream.is_stopped(),
            Self::Output(stream) => stream.is_stopped(),
        }
    }

    async fn stop(&mut self) -> Result<(), anyhow::Error> {
        match self {
            Self::Input(stream) => stream.stop().await,
            Self::Output(stream) => stream.stop().await,
        }
    }

    async fn start(&mut self) -> Result<(), anyhow::Error> {
        match self {
            Self::Input(stream) => stream.start().await,
            Self::Output(stream) => stream.start().await,
        }
    }

    async fn metadata(&mut self, key: String, value: String) -> Result<(), anyhow::Error> {
        match self {
            Self::Input(stream) => stream.metadata(key, value).await,
            Self::Output(stream) => stream.metadata(key, value).await
        }        
    }
}
