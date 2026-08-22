//! Micro-benchmark for the per-audio-frame cost the routing hot-path guard
//! removes: cloning the full packet and feeding the clone to
//! `CacheManager::process_packet`, whose AudioFrame arm is a no-op.
//!
//! `route_bench` drives `route_audio_frame` directly and never touches the two
//! packet loops in `quic/mod.rs`, so it cannot see this cost. This bench isolates
//! exactly what the guard eliminates: one deep `QuicNetworkPacket::clone` (the
//! Opus payload plus the sender `PlayerEnum` with its owned Strings) and one
//! awaited call that immediately falls through.
//!
//!   cargo run --release --example frame_clone_bench -- --iterations 2000000
//!
//! Run it once before the guard lands to record the removed per-frame cost, and
//! project it to a full server: cost_removed_per_second = per_frame × 50 × speakers.

use std::time::Instant;

use bvc_server_lib::stream::quic::CacheManager;
use clap::Parser;
use common::game_data::Dimension;
use common::players::MinecraftPlayer;
use common::structs::packet::{
    AudioFramePacket, PacketSender, PacketType, QuicNetworkPacket, QuicNetworkPacketData,
};
use common::{Coordinate, Orientation, PlayerEnum};

#[derive(Debug, Parser)]
#[clap(about = "Measure the per-frame clone + no-op process_packet cost the routing guard removes")]
struct Args {
    /// Iterations to time (each is one simulated audio frame through the loop)
    #[clap(long, default_value = "2000000")]
    iterations: u64,

    /// Speakers to project the per-second server-wide saving onto (at 50 fps each)
    #[clap(long, default_value = "50")]
    speakers: u64,
}

struct CloneBench {
    args: Args,
    cache_manager: CacheManager,
    packet: QuicNetworkPacket,
}

impl CloneBench {
    fn new(args: Args) -> Self {
        let sender = PlayerEnum::Minecraft(MinecraftPlayer {
            name: "SpeakerPlayer".to_string(),
            coordinates: Coordinate { x: 12.0, y: 64.0, z: -40.0 },
            orientation: Orientation { x: 90.0, y: 12.0 },
            dimension: Dimension::Overworld,
            deafen: false,
            spectator: false,
            world_uuid: Some("00000000-0000-0000-0000-000000000000".to_string()),
            alternative_identity: None,
            player_uuid: Some("11111111-1111-1111-1111-111111111111".to_string()),
            relay_world_uuid: None,
            bridged_voice: false,
        });

        let packet = QuicNetworkPacket {
            packet_type: PacketType::AudioFrame,
            sender: Some(PacketSender::player(
                common::Game::Minecraft.membership_key("SpeakerPlayer"),
                7,
            )),
            // 160 bytes ~= one 20ms Opus frame at 64kbps
            data: QuicNetworkPacketData::AudioFrame(AudioFramePacket::new(
                vec![0u8; 160],
                Some(sender),
                Some(true),
            )),
            // Not a server fan-out to one connection, so this envelope carries no sequence.
            ..Default::default()
        };

        Self {
            args,
            cache_manager: CacheManager::new(),
            packet,
        }
    }

    // The removed path: clone the packet and feed the clone to process_packet,
    // exactly as both quic/mod.rs loops do today for every audio frame.
    async fn removed_path_cost_ns(&self) -> f64 {
        let warmup = (self.args.iterations / 20).max(1);
        for _ in 0..warmup {
            let clone = self.packet.clone();
            let _ = self.cache_manager.process_packet(clone).await;
        }

        let started = Instant::now();
        for _ in 0..self.args.iterations {
            let clone = self.packet.clone();
            let _ = self.cache_manager.process_packet(clone).await;
        }
        started.elapsed().as_nanos() as f64 / self.args.iterations as f64
    }

    // The kept path: the guard's `packet_type != AudioFrame` check, which is all
    // that runs for an audio frame after the change. std::hint::black_box keeps
    // the comparison from being optimized away.
    fn kept_path_cost_ns(&self) -> f64 {
        let started = Instant::now();
        let mut hits = 0u64;
        for _ in 0..self.args.iterations {
            if std::hint::black_box(self.packet.packet_type != PacketType::AudioFrame) {
                hits += 1;
            }
        }
        std::hint::black_box(hits);
        started.elapsed().as_nanos() as f64 / self.args.iterations as f64
    }

    async fn run(self) {
        let removed = self.removed_path_cost_ns().await;
        let kept = self.kept_path_cost_ns();
        let saved = (removed - kept).max(0.0);

        let per_speaker_per_sec_us = saved * 50.0 / 1_000.0;
        let server_per_sec_us = per_speaker_per_sec_us * self.args.speakers as f64;

        println!("--- frame_clone_bench results ---");
        println!("iterations:              {}", self.args.iterations);
        println!("removed path (clone + process_packet): {:.1} ns/frame", removed);
        println!("kept path (type check only):           {:.1} ns/frame", kept);
        println!("net saved per frame:                   {:.1} ns", saved);
        println!(
            "projected per speaker @50fps:          {:.2} µs/s",
            per_speaker_per_sec_us
        );
        println!(
            "projected server-wide @{} speakers:    {:.1} µs/s ({:.4}% of one core)",
            self.args.speakers,
            server_per_sec_us,
            server_per_sec_us / 10_000.0
        );
    }
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    CloneBench::new(args).run().await;
}
