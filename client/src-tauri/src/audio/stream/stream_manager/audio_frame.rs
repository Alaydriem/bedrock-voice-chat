use super::AudioFrameData;

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

