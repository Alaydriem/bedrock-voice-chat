use serde::{Deserialize, Serialize};

pub mod input;
pub mod output;

pub use input::InputRecordingHeader;
pub use output::OutputRecordingHeader;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RecordingHeader {
    Input(InputRecordingHeader),
    Output(OutputRecordingHeader),
}

impl RecordingHeader {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, postcard::Error> {
        postcard::from_bytes(bytes)
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, postcard::Error> {
        postcard::to_allocvec(self)
    }

    pub fn sample_rate(&self) -> u32 {
        match self {
            RecordingHeader::Input(header) => header.sample_rate,
            RecordingHeader::Output(header) => header.sample_rate,
        }
    }

    pub fn channels(&self) -> u16 {
        match self {
            RecordingHeader::Input(header) => header.channels,
            RecordingHeader::Output(header) => header.channels,
        }
    }

    pub fn is_spatial(&self) -> bool {
        match self {
            RecordingHeader::Input(_) => false,
            RecordingHeader::Output(header) => header.is_spatial,
        }
    }
}
