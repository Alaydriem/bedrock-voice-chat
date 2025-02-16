mod input;
mod output;
mod stream_trait;

use common::structs::audio::AudioDevice;

pub(crate) use stream_trait::StreamTrait;
pub(crate) use input::InputStream;
pub(crate) use output::OutputStream;

#[derive(Debug, Clone)]
pub(crate) enum AudioFrame {
    F32(AudioFrameData<f32>),
    I32(AudioFrameData<i32>),
    I16(AudioFrameData<i16>)
}

#[derive(Debug, Clone)]
pub(crate) struct AudioFrameData<T> {
    pcm: Vec<T>
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) enum IpcMessage {
    Terminate
}

pub(crate) enum StreamTraitType {
    Input(InputStream),
    Output(OutputStream)
}

impl StreamTrait for StreamTraitType {
    fn is_stopped(&mut self) -> bool {
        match self {
            Self::Input(stream) => stream.is_stopped(),
            Self::Output(stream) => stream.is_stopped()
        }
    }

    fn stop(&mut self) {
        match self {
            Self::Input(stream) => stream.stop(),
            Self::Output(stream) => stream.stop()
        }
    }

    fn start(&mut self) -> Result<(), anyhow::Error> {
        match self {
            Self::Input(stream) => stream.start(),
            Self::Output(stream) => stream.start()
        }
    }
}

impl StreamTraitType {
    pub fn get_device(&self) -> Option<AudioDevice> {
        match self {
            Self::Input(stream) => stream.device.clone(),
            Self::Output(stream) => stream.device.clone()
        }
    }
}
