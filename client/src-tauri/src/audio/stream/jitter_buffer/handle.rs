use super::EncodedAudioFramePacket;
use super::source_error::JitterBufferError;

#[derive(Clone)]
pub struct JitterBufferHandle {
    tx: flume::Sender<Option<EncodedAudioFramePacket>>,
}

impl JitterBufferHandle {
    pub(super) fn new(tx: flume::Sender<Option<EncodedAudioFramePacket>>) -> Self {
        Self { tx }
    }

    pub fn enqueue(&self, packet: EncodedAudioFramePacket) -> Result<(), JitterBufferError> {
        self.tx
            .send(Some(packet))
            .map_err(|_| JitterBufferError::InvalidPacket)
    }

    pub fn stop(&self) {
        // Send None to indicate stop
        let _ = self.tx.send(None);
    }
}
