use moka::sync::Cache;
use rodio::{Sink, SpatialSink};
use rodio::mixer::Mixer;
use std::sync::Arc;
use std::time::Duration;
use log::{info, error};
use common::Player;
use common::structs::audio::PlayerGainStore;
use std::sync::mpsc::Receiver;
use crate::audio::stream::jitter_buffer::DecodedAudioFramePacket;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::AppHandle;
use tokio::task::JoinHandle;
use super::audio_sink::{AudioSink, AudioSinkType, AudioSinkTarget};

pub(crate) struct SinkManager {
    sinks: Cache<Vec<u8>, Arc<AudioSink>>,
    current_player_name: String,
    #[allow(unused)]
    app_handle: AppHandle,
    mixer: Arc<Mixer>,
    players: Arc<moka::sync::Cache<String, Player>>,
    player_gain_store: PlayerGainStore,
    consumer: Option<Receiver<DecodedAudioFramePacket>>,
    shutdown: Arc<AtomicBool>,
    player_store_update_available: Arc<AtomicBool>
}

impl SinkManager {
    pub fn new(
        app_handle: AppHandle,
        mixer: Arc<Mixer>,
        players: Arc<moka::sync::Cache<String, Player>>,
        player_gain_store: PlayerGainStore,
        current_player_name: String,
        consumer: Receiver<DecodedAudioFramePacket>,
    ) -> Self {
        Self {
            sinks: Cache::builder()
                .time_to_idle(Duration::from_secs(15 * 60))
                .build(),
            current_player_name,
            app_handle,
            mixer,
            players,
            player_gain_store,
            consumer: Some(consumer),
            shutdown: Arc::new(AtomicBool::new(false)),
            player_store_update_available: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn update_player_store(&mut self, player_gain_store: PlayerGainStore) {
        self.player_gain_store = player_gain_store;
        _ = self.player_store_update_available.store(true, Ordering::Relaxed);
    }

    pub async fn listen(&mut self) -> Result<JoinHandle<()>, anyhow::Error> {
        _ = self.shutdown.store(false, Ordering::Relaxed);

        let shutdown = self.shutdown.clone();
        let mixer = self.mixer.clone();
        let consumer = self.consumer.take().ok_or_else(|| anyhow::anyhow!("SinkManager listener already started"))?;

        let handle = tokio::spawn(async move {
            #[allow(irrefutable_let_patterns)]
            while let packet = consumer.recv() {
                if shutdown.load(Ordering::Relaxed) {
                    break;
                }

                match packet {
                    Ok(packet) => {
                        let owner = match packet.owner {
                            Some(o) => o,
                            None => continue,
                        };                        

                        // TODO: Append the packet to the correct player's jitter buffer / sink queue
                    }
                    Err(_e) => {
                        break;
                    }
                }
            }
        });

        Ok(handle)
    }

    pub async fn stop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);

        // Give existing jobs 500ms to clear
        tokio::time::sleep(Duration::from_millis(500)).await;

        for (_, sink) in self.sinks.iter() {
            sink.clear_and_stop();
        }

        info!("SinkManager has been stopped.");
    }

    pub fn get_sink(
        &self,
        player_id: Vec<u8>,
        sink_type: AudioSinkType,
        sample_rate: u32,
    ) -> Arc<AudioSink> {
        self.sinks.get(&player_id).unwrap_or_else(|| {
            let new_sink = match sink_type {
                AudioSinkType::Normal => {
                    let rodio_sink = Arc::new(Sink::connect_new(&self.mixer));
                    AudioSink::new(
                        AudioSinkTarget::Normal(rodio_sink),
                        opus::Decoder::new(sample_rate, opus::Channels::Mono).unwrap(),
                    )
                }
                AudioSinkType::Spatial => {
                    let rodio_sink = Arc::new(SpatialSink::connect_new(
                        &self.mixer,
                        [0.0, 0.0, 0.0],
                        [0.0, 0.0, 0.0],
                        [0.0, 0.0, 0.0],
                    ));
                    AudioSink::new(
                        AudioSinkTarget::Spatial(rodio_sink),
                        opus::Decoder::new(sample_rate, opus::Channels::Mono).unwrap(),
                    )
                }
            };
            let sink = Arc::new(new_sink);
            self.sinks.insert(player_id.clone(), sink.clone());
            sink
        })
    }
}
