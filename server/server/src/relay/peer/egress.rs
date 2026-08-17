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
                sample_rate: audio.sample_rate,
                opus: audio.data.to_vec(),
                timestamp_ms: audio.timestamp(),
                spatial: audio.spatial.unwrap_or(false),
                jukebox,
            },
        ))
    }
}
