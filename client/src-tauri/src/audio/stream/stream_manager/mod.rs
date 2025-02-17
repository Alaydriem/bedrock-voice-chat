mod input;
mod output;

use common::structs::audio::AudioDevice;

pub(crate) use input::InputStream;
pub(crate) use output::OutputStream;
pub(crate) use crate::core::StreamTrait;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub(crate) enum AudioFrame {
    F32(AudioFrameData<f32>),
    I32(AudioFrameData<i32>),
    I16(AudioFrameData<i16>),
}

impl AudioFrame {
    pub fn f32(self) -> Option<AudioFrameData<f32>> {
        if let AudioFrame::F32(f) = self {
            return Some(f);
        }

        None
    }

    #[allow(unused)]
    pub fn i32(self) -> Option<AudioFrameData<i32>> {
        if let AudioFrame::I32(f) = self {
            return Some(f);
        }

        None
    }

    #[allow(unused)]
    pub fn i16(self) -> Option<AudioFrameData<i16>> {
        if let AudioFrame::I16(f) = self {
            return Some(f);
        }

        None
    }
}

#[derive(Debug, Clone)]
pub(crate) struct AudioFrameData<T> {
    pcm: Vec<T>,
}

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

impl StreamTraitType {
    pub fn get_device(&self) -> Option<AudioDevice> {
        match self {
            Self::Input(stream) => stream.device.clone(),
            Self::Output(stream) => stream.device.clone(),
        }
    }
}
