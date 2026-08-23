use common::curia;
use std::time::Duration;

use common::game_data::Dimension;
use common::structs::packet::{
    AudioFrameMetadata, AudioFramePacket, JukeboxMetadata, PacketSender, PacketType,
    SpeakerPosition,
    QuicNetworkPacket, QuicNetworkPacketData,
};
use common::{Coordinate, PlayerEnum};
use tokio_util::sync::CancellationToken;

use crate::stream::quic::WebhookReceiver;

pub struct PlaybackTask {
    event_id: String,
    jukebox_name: String,
    position: Coordinate,
    dimension: Dimension,
    frames: Vec<Vec<u8>>,
    webhook_receiver: WebhookReceiver,
    synthetic_player: PlayerEnum,
    cancel_token: CancellationToken,
}

impl PlaybackTask {
    pub fn new(
        event_id: String,
        jukebox_name: String,
        position: Coordinate,
        dimension: Dimension,
        frames: Vec<Vec<u8>>,
        webhook_receiver: WebhookReceiver,
        synthetic_player: PlayerEnum,
        cancel_token: CancellationToken,
    ) -> Self {
        Self {
            event_id,
            jukebox_name,
            position,
            dimension,
            frames,
            webhook_receiver,
            synthetic_player,
            cancel_token,
        }
    }

    pub async fn run(self) {
        let total_frames = self.frames.len();
        curia::debug!("Playback task starting", { "event_id": self.event_id.to_string(), "total_frames": total_frames });

        let start = tokio::time::Instant::now();
        let mut sent = 0usize;

        // The jukebox name already carries a slice of the event id, so concurrent playbacks
        // stay distinguishable without a device id — and a device id would be a lie, because
        // no connection is speaking.
        let packet_sender = PacketSender::for_service(self.jukebox_name.clone());

        for (i, frame) in self.frames.iter().enumerate() {
            let next_tick = start + Duration::from_millis(20 * i as u64);
            tokio::select! {
                _ = self.cancel_token.cancelled() => {
                    curia::debug!("Playback cancelled", { "event_id": self.event_id.to_string(), "sent": sent });
                    return;
                }
                _ = tokio::time::sleep_until(next_tick) => {
                    let metadata = vec![AudioFrameMetadata::Jukebox(JukeboxMetadata::new(
                        self.position.clone(),
                        self.event_id.clone(),
                        self.dimension.clone(),
                    ))];
                    let audio_frame = AudioFramePacket::new(
                        frame.clone(),
                        Some(SpeakerPosition::from_player(&self.synthetic_player)),
                        Some(true),
                    )
                    .with_metadata(metadata);

                    let packet = QuicNetworkPacket {
                        packet_type: PacketType::AudioFrame,
                        sender: Some(packet_sender.clone()),
                        data: QuicNetworkPacketData::AudioFrame(audio_frame),
                                            // Not a server fan-out, so this envelope carries no sequence.
                        ..Default::default()
                    };

                    let result: Result<(), Box<dyn std::error::Error>> =
                        self.webhook_receiver.send_packet(packet).await;
                    if let Err(e) = result {
                        curia::error!("Failed to send playback frame, aborting", { "event_id": self.event_id.to_string(), "frame": i, "error": e.to_string() });
                        return;
                    }
                    sent += 1;
                }
            }
        }

        let elapsed = start.elapsed();
        curia::debug!("Playback task completed", { "event_id": self.event_id.to_string(), "sent": sent, "total_frames": total_frames, "elapsed_ms": elapsed.as_millis() as u64 });
    }
}
