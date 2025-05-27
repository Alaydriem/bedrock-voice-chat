mod input;
mod output;
mod sink_manager;

use std::sync::Arc;

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

impl StreamTraitType {
    pub fn get_device(&self) -> Option<AudioDevice> {
        match self {
            Self::Input(stream) => stream.device.clone(),
            Self::Output(stream) => stream.device.clone(),
        }
    }

    pub fn get_metadata(&self) -> Arc<moka::future::Cache<String, String>> {
        match self {
            Self::Input(stream) => stream.metadata.clone(),
            Self::Output(stream) => stream.metadata.clone(),
        }
    }

    pub fn mute(&self) {
        match self {
            Self::Input(stream) => stream.mute(),
            Self::Output(stream) => stream.mute(),
        }
    }

    pub fn mute_status(&self) -> bool {
        match self {
            Self::Input(stream) => stream.mute_status(),
            Self::Output(stream) => stream.mute_status(),
        }
    }
}
