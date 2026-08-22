use common::structs::packet::{AudioFrameMetadata, QuicNetworkPacket, QuicNetworkPacketData};
use common::structs::relay::wire::datagram::VoiceFrame;
use common::traits::player_data::PlayerData;

// The inverse of `PeerIngest`: a local packet becomes a wire frame, or does not
// leave at all.
//
// `None` rather than an error for every non-forwardable case. This runs on the
// audio path for every local frame, and "this is not peer traffic" is the common
// answer rather than a fault.
pub struct PeerEgress;

// Every producer in this codebase encodes Opus at 48 kHz, and the audio frame no longer
// carries a rate to copy. The peer wire keeps its `sample_rate` field, which is a published
// format, so the value is supplied here rather than dropped from it.
const OPUS_SAMPLE_RATE: u32 = 48_000;

impl PeerEgress {
    pub fn frame_from(packet: &QuicNetworkPacket) -> Option<(String, VoiceFrame)> {
        let QuicNetworkPacketData::AudioFrame(audio) = &packet.data else {
            return None;
        };

        let speaker = audio.sender.as_ref()?;
        let world = speaker.world_identifier()?.to_string();

        // The playback id, when this is jukebox audio. Read from the metadata the
        // playback task attached rather than parsed back out of the speaker's
        // name, so the far side is told rather than left to infer.
        let jukebox = audio.metadata.iter().find_map(|meta| match meta {
            AudioFrameMetadata::Jukebox(jb) => Some(jb.event_id.clone()),
        });

        Some((
            world,
            VoiceFrame {
                speaker: speaker.clone(),
                sample_rate: OPUS_SAMPLE_RATE,
                opus: audio.data.to_vec(),
                timestamp_ms: audio.timestamp(),
                spatial: audio.spatial.unwrap_or(false),
                jukebox,
            },
        ))
    }
}
