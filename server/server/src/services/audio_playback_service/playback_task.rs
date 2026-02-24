use std::time::Duration;

use common::structs::packet::{
    AudioFramePacket, PacketOwner, PacketType, QuicNetworkPacket, QuicNetworkPacketData,
};
use common::PlayerEnum;
use tokio_util::sync::CancellationToken;

use crate::stream::quic::WebhookReceiver;

/// Manages streaming pre-encoded Opus frames at 20ms intervals over QUIC.
/// Coordinates are embedded in the synthetic MinecraftPlayer inside each AudioFramePacket.sender.
pub struct PlaybackTask {
    event_id: String,
    frames: Vec<Vec<u8>>,
    webhook_receiver: WebhookReceiver,
    /// Synthetic player at the block's world coordinates.
    /// Coordinates are read by is_receivable() for spatial filtering on the client.
    synthetic_player: PlayerEnum,
    cancel_token: CancellationToken,
}

impl PlaybackTask {
    pub fn new(
        event_id: String,
        frames: Vec<Vec<u8>>,
        webhook_receiver: WebhookReceiver,
        synthetic_player: PlayerEnum,
        cancel_token: CancellationToken,
    ) -> Self {
        Self {
            event_id,
            frames,
            webhook_receiver,
            synthetic_player,
            cancel_token,
        }
    }

    /// Run the playback loop. Epoch-anchored timing prevents drift.
    pub async fn run(self) {
        let total_frames = self.frames.len();
        tracing::debug!(
            event_id = %self.event_id,
            total_frames = total_frames,
            "Playback task starting"
        );

        let start = tokio::time::Instant::now();
        let mut sent = 0usize;

        for (i, frame) in self.frames.iter().enumerate() {
            let next_tick = start + Duration::from_millis(20 * i as u64);
            tokio::select! {
                _ = self.cancel_token.cancelled() => {
                    tracing::debug!(
                        event_id = %self.event_id,
                        sent = sent,
                        "Playback cancelled"
                    );
                    return;
                }
                _ = tokio::time::sleep_until(next_tick) => {
                    let audio_frame = AudioFramePacket::new(
                        frame.clone(),
                        48000,
                        Some(self.synthetic_player.clone()),
                        Some(true),
                    );

                    let packet = QuicNetworkPacket {
                        packet_type: PacketType::AudioFrame,
                        owner: Some(PacketOwner {
                            name: format!("jukebox::{}", self.event_id),
                            client_id: self.event_id.as_bytes().to_vec(),
                        }),
                        data: QuicNetworkPacketData::AudioFrame(audio_frame),
                    };

                    let result: Result<(), Box<dyn std::error::Error>> =
                        self.webhook_receiver.send_packet(packet).await;
                    if let Err(e) = result {
                        tracing::error!(
                            event_id = %self.event_id,
                            frame = i,
                            error = %e,
                            "Failed to send playback frame, aborting"
                        );
                        return;
                    }
                    sent += 1;
                }
            }
        }

        let elapsed = start.elapsed();
        tracing::debug!(
            event_id = %self.event_id,
            sent = sent,
            total_frames = total_frames,
            elapsed_ms = elapsed.as_millis() as u64,
            "Playback task completed"
        );
    }
}
