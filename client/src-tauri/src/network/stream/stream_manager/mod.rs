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

    fn stop(&mut self) {
        match self {
            Self::Input(stream) => stream.stop(),
            Self::Output(stream) => stream.stop(),
        }
    }

    fn start(&mut self) -> Result<(), anyhow::Error> {
        match self {
            Self::Input(stream) => stream.start(),
            Self::Output(stream) => stream.start(),
        }
    }
}
