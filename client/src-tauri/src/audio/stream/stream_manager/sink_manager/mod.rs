use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use flume::Receiver;
use log::{info, warn};
use moka::sync::Cache;
use rodio::{Player, mixer::Mixer};
use tokio::task::JoinHandle;

use crate::audio::recording::RecordingProducer;
use crate::audio::spatial::{PerceptualGain, SpatialCalculator, SpatialGains};
use crate::audio::stream::ActivityUpdate;
use crate::audio::stream::jitter_buffer::{EncodedAudioFramePacket, JitterBuffer, PanState};
use crate::audio::stream::level_bus::LoudnessTracker;
use crate::audio::stream::stream_manager::audio_sink::AudioSink;
use crate::audio::stream::stream_manager::mono_to_panned::MonoToPanned;
use crate::diagnostics::{PeerRegistry, PeerRoute, PlayerReceiveStats};
use common::PlayerEnum;
use common::structs::SpatialAudioConfig;
use common::structs::audio::{GainProjection, JukeboxLevel, PlayerGainSettings};
use common::traits::player_data::PlayerData;

mod mono_to_stereo;
mod player_sinks;

use mono_to_stereo::MonoToStereo;
use player_sinks::PlayerSinks;


pub struct SinkManager {
    consumer: Option<Receiver<EncodedAudioFramePacket>>,
    shutdown: Arc<AtomicBool>,
    global_mute: Arc<AtomicBool>,
    panning_intensity: Arc<AtomicU32>,
    players: Cache<String, PlayerEnum>,
    current_player_name: String,
    gain: Arc<GainProjection>,
    // The player's one opinion about jukebox music. Consulted per sink, so concurrent playbacks
    // each resolve their own volume from it rather than sharing anything.
    jukebox: Arc<JukeboxLevel>,
    // Keyed on `EncodedAudioFramePacket::sink_key` — the emitter's device id, or its
    // name when the server injected the audio.
    sinks: Cache<String, PlayerSinks>,
    mixer: Arc<Mixer>,
    activity_tx: Option<flume::Sender<ActivityUpdate>>,
    #[allow(unused)]
    app_handle: tauri::AppHandle,
    recording_producer: Option<RecordingProducer>,
    recording_active: Option<Arc<AtomicBool>>,
    spatial_config: SpatialAudioConfig,
    peer_registry: Arc<PeerRegistry>,
    sweep_handle: Option<JoinHandle<()>>,
}

impl SinkManager {
    // Negate pan on platforms where the audio backend outputs channels
    // in the opposite order to what we expect ([R,L] instead of [L,R]).
    // Test each platform and flip the sign here if panning is inverted.
    fn platform_adjusted_pan(pan: f32) -> f32 {
        pan
    }

    // How often quiet jukebox sinks are looked for, and how many consecutive quiet passes retire
    // one. Two passes rather than one so a scheduling hiccup that starves the audio thread for a
    // moment cannot retire a sink that is still playing.
    const SWEEP_INTERVAL: Duration = Duration::from_secs(5);
    const QUIET_PASSES_TO_RETIRE: u32 = 2;

    pub fn new(
        consumer: Receiver<EncodedAudioFramePacket>,
        players: Cache<String, PlayerEnum>,
        current_player_name: String,
        gain: Arc<GainProjection>,
        mixer: Arc<Mixer>,
        app_handle: tauri::AppHandle,
        recording_producer: Option<RecordingProducer>,
        recording_active: Option<Arc<AtomicBool>>,
        spatial_config: SpatialAudioConfig,
        panning_intensity: f32,
        peer_registry: Arc<PeerRegistry>,
        jukebox: Arc<JukeboxLevel>,
        levels: Arc<crate::audio::stream::level_bus::LevelBus>,
    ) -> Self {
        // Create activity streaming channel
        let (activity_tx, activity_rx) = flume::unbounded::<ActivityUpdate>();

        // Peer activity folded into the shared bus rather than emitted from here.
        //
        // This used to be a second emitter on its own 100 ms timer, publishing `audio-activity`
        // alongside the capture path's `audio-input-level`. Two webview messages every tenth of
        // a second, for information that is always read together — and on Android each of those
        // is a unit of main-thread work competing with the rendering of the very meters they
        // feed. Now the levels are collected here and one publisher decides when they are worth
        // a message.
        //
        // The trackers are held per peer so a steady voice stops changing its step. Without
        // them a peer sitting on a boundary would flip between two values every frame, and a
        // changed value is what buys a message.
        let bus = levels.clone();
        tokio::spawn(async move {
            let mut trackers: std::collections::HashMap<String, LoudnessTracker> =
                std::collections::HashMap::new();

            while let Ok(update) = activity_rx.recv_async().await {
                // The roster is people. A jukebox is a synthetic speaker with no card, no
                // presence and no gain of its own, and both webview consumers of this snapshot
                // mint an entry for every name in it. Its counters still reach the diagnostics
                // registry, which is a different feed for a different reader.
                if update
                    .player_name
                    .starts_with(common::consts::audio::JUKEBOX_PLAYER_PREFIX)
                {
                    continue;
                }

                let tracker = trackers.entry(update.player_name.clone()).or_default();
                // A peer's frame reaching the mixer at all is a frame that was decoded and
                // played, so it is audible by construction; only the amplitude is in question.
                let level = tracker.observe(update.rms_level, update.rms_level > 0.0);
                bus.set_peer(update.player_name, level);
            }
        });

        Self {
            consumer: Some(consumer),
            shutdown: Arc::new(AtomicBool::new(false)),
            global_mute: Arc::new(AtomicBool::new(false)),
            panning_intensity: Arc::new(AtomicU32::new(
                panning_intensity.clamp(0.0, 1.0).to_bits(),
            )),
            players,
            current_player_name,
            gain,
            jukebox,
            sinks: Cache::builder()
                // 15 minutes
                .time_to_live(Duration::from_secs(15 * 60))
                .max_capacity(100)
                .build(),
            mixer,
            activity_tx: Some(activity_tx),
            app_handle,
            recording_producer,
            recording_active,
            spatial_config,
            peer_registry,
            sweep_handle: None,
        }
    }

    // Retires jukebox sinks whose frames have stopped.
    //
    // A jukebox sink is per playback, not per block, so nothing ever writes to it again once the
    // disc is out and the entry would otherwise sit in both caches for fifteen minutes. Frames
    // stopping is the one signal present in every case: a hand-pulled disc, the server's
    // auto-eject, and the addon's own stop over HTTP all end with the playback task cancelled.
    //
    // Runs on its own task at a fraction of a hertz. It is deliberately not folded into the
    // packet loop, which must not take on work that has nothing to do with the frame in hand.
    fn spawn_retirement_sweep(
        sinks: Cache<String, PlayerSinks>,
        peer_registry: Arc<PeerRegistry>,
        shutdown: Arc<AtomicBool>,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            let mut last_seen: std::collections::HashMap<String, (u64, u32)> =
                std::collections::HashMap::new();

            loop {
                tokio::time::sleep(Self::SWEEP_INTERVAL).await;
                if shutdown.load(Ordering::Relaxed) {
                    break;
                }

                let counts = peer_registry.jukebox_frame_counts();
                last_seen.retain(|key, _| counts.iter().any(|(live, _)| live == key));

                for (sink_key, received) in counts {
                    let (previous, quiet_passes) =
                        last_seen.get(&sink_key).copied().unwrap_or((received, 0));

                    let quiet_passes = if received == previous {
                        quiet_passes + 1
                    } else {
                        0
                    };

                    if quiet_passes >= Self::QUIET_PASSES_TO_RETIRE {
                        Self::retire_sink(&sinks, &peer_registry, &sink_key);
                        last_seen.remove(&sink_key);
                        continue;
                    }

                    last_seen.insert(sink_key, (received, quiet_passes));
                }
            }
        })
    }

    // Stops a sink's jitter buffers, silences and drops its mixer sinks, and removes its
    // diagnostics rows. The bundle is invalidated last: the handles have to be stopped while
    // they are still reachable.
    fn retire_sink(
        sinks: &Cache<String, PlayerSinks>,
        peer_registry: &Arc<PeerRegistry>,
        sink_key: &str,
    ) {
        if let Some(bundle) = sinks.get(sink_key) {
            if let Some(h) = &bundle.normal_handle {
                h.stop();
            }
            if let Some(h) = &bundle.spatial_handle {
                h.stop();
            }
            if let Some(s) = &bundle.normal {
                s.clear_and_stop();
            }
            if let Some(s) = &bundle.spatial {
                s.clear_and_stop();
            }
            sinks.invalidate(sink_key);
        }

        peer_registry.unregister(sink_key);
        info!("Retired jukebox sink {}", sink_key);
    }

    pub fn update_global_mute(&self, muted: bool) {
        self.global_mute.store(muted, Ordering::Relaxed);
    }

    pub fn update_panning_intensity(&self, intensity: f32) {
        self.panning_intensity
            .store(intensity.clamp(0.0, 1.0).to_bits(), Ordering::Relaxed);
    }

    pub async fn listen(&mut self) -> Result<JoinHandle<()>, anyhow::Error> {
        _ = self.shutdown.store(false, Ordering::Relaxed);

        let shutdown = self.shutdown.clone();
        let consumer = self
            .consumer
            .take()
            .ok_or_else(|| anyhow::anyhow!("SinkManager listener already started"))?;
        let players = self.players.clone();
        let current_player_name = self.current_player_name.clone();
        let gain = self.gain.clone();
        let jukebox = self.jukebox.clone();
        let sinks = self.sinks.clone();
        let mixer = self.mixer.clone();
        let global_mute = self.global_mute.clone();
        let panning_intensity = self.panning_intensity.clone();
        let activity_tx = self.activity_tx.clone();
        let recording_producer = self.recording_producer.clone();
        let recording_active = self.recording_active.clone();
        let spatial_config = self.spatial_config.clone();
        let peer_registry = self.peer_registry.clone();
        // Read once per listener rather than per sink: a session cannot change transport
        // while it is up, and a fresh sink mid-session must not be given a different
        // buffer floor from the sinks beside it. A reconnect rebuilds this listener.
        let transport = {
            use tauri::Manager;
            self.app_handle
                .try_state::<std::sync::Arc<crate::diagnostics::LinkSession>>()
                .and_then(|session| session.transport())
                .unwrap_or(common::structs::metrics::TransportKind::Quic)
        };

        // Spawn an async task; use async recv to avoid blocking
        let handle = tokio::spawn(async move {
            while let Ok(packet) = consumer.recv_async().await {
                if shutdown.load(Ordering::Relaxed) {
                    break;
                }

                let sink_key = packet.sink_key();

                // Before the mute check below, which drops the frame. Counted here, "is music
                // playing" stays true while muted; counted after, muting would read as the disc
                // having ended.
                if sink_key.starts_with(common::consts::audio::JUKEBOX_PLAYER_PREFIX) {
                    peer_registry.note_jukebox_frame();
                }

                // The speaker's canonical `game:gamertag`, or the sink key when the server
                // injected the audio and there is no player behind it. This is what the
                // activity event carries, so a meter in the webview keys on the same identity
                // as the card it sits on.
                let identity = match packet.emitter.name.as_str() {
                    name if !name.is_empty()
                        && name != common::structs::packet::PacketSender::SERVER_API =>
                    {
                        name.to_string()
                    }
                    _ => sink_key.clone(),
                };

                // Display only — this is what a support log line and the diagnostics table
                // name a speaker as, and a human reads "Alice", not "minecraft:Alice". Every
                // lookup on this path uses `sink_key` or `identity` instead.
                let display_name = common::Game::display_name(&identity).to_string();

                let emitter_pos = packet
                    .emitter
                    .player_data
                    .as_ref()
                    .map(|p| p.get_position().clone());
                let deafen_emitter = packet
                    .emitter
                    .player_data
                    .as_ref()
                    .map(|p| p.is_deafened())
                    .unwrap_or(false);
                let emitter_spatial = packet.emitter.spatial.unwrap_or(true);

                let listener_info = players.get(&current_player_name).map(|player| {
                    let pos = player.get_position().clone();
                    let orient = player.get_orientation().clone();
                    (pos, orient)
                });

                if listener_info.is_none() {
                    log::debug!(
                        "Listener '{}' not found in player cache (cache size: {})",
                        current_player_name,
                        players.entry_count(),
                    );
                }

                let use_spatial =
                    emitter_spatial && listener_info.is_some() && emitter_pos.is_some();

                // A synthetic emitter has no device and so no per-player opinion. A jukebox takes
                // the player's one music setting, resolved per sink so concurrent playbacks stay
                // independent; anything else synthetic — channel API audio — takes unity, because
                // a music control must not silence an announcement.
                let gain_settings: PlayerGainSettings = match packet.emitter.device {
                    Some(device) => gain.settings_for(device),
                    None => jukebox.settings_for(&sink_key),
                };
                if gain_settings.muted {
                    continue;
                }

                let mut bundle = sinks.get(&sink_key).unwrap_or_else(|| {
                    let b = PlayerSinks::default();
                    if let Some(existing) = sinks.get(&sink_key) {
                        existing
                    } else {
                        sinks.insert(sink_key.clone(), b.clone());
                        b
                    }
                });

                if use_spatial {
                    if bundle.spatial.is_none() {
                        let rodio_sink = Arc::new(Player::connect_new(&mixer));
                        let sink = Arc::new(AudioSink::Spatial(rodio_sink));
                        sink.play();
                        bundle.spatial = Some(sink);
                        bundle.spatial_pan_state = Some(Arc::new(PanState::new()));
                    }

                    let (listener_coordinate, listener_orientation) = listener_info.unwrap();
                    let emitter_coordinate = emitter_pos.unwrap();

                    let listener_player = players.get(&current_player_name);
                    let game = listener_player
                        .as_ref()
                        .map(|p| p.get_game())
                        .unwrap_or(common::Game::Minecraft);

                    let spatial_data = SpatialCalculator::gains(
                        &emitter_coordinate,
                        deafen_emitter,
                        &listener_coordinate,
                        &listener_orientation,
                        game,
                        &spatial_config,
                    );

                    if let Some(pan_state) = &bundle.spatial_pan_state {
                        let mute_mult = if global_mute.load(Ordering::Relaxed) {
                            0.0
                        } else {
                            1.0
                        };
                        let volume =
                            spatial_data.volume
                                * PerceptualGain::amplitude(gain_settings.gain)
                                * mute_mult;

                        let intensity = f32::from_bits(panning_intensity.load(Ordering::Relaxed));
                        let gains = SpatialGains::from_pan(
                            Self::platform_adjusted_pan(spatial_data.pan),
                            volume,
                            intensity,
                        );
                        pan_state.update(gains.left, gains.right, gains.volume);
                    }

                    if bundle.spatial_handle.is_none() {
                        let stats = Arc::new(PlayerReceiveStats::new(display_name.clone()));
                        peer_registry.register(
                            sink_key.clone(),
                            PeerRoute::Spatial,
                            stats.clone(),
                        );
                        match JitterBuffer::create_with_handle_and_activity(
                            packet.clone(),
                            format!("spatial_{}", sink_key),
                            identity.clone(),
                            activity_tx.clone(),
                            recording_producer.clone(),
                            recording_active.clone(),
                            stats,
                            transport,
                        ) {
                            Ok((jitter_buffer, handle)) => {
                                if let (Some(spatial_sink), Some(pan_state)) =
                                    (&bundle.spatial, &bundle.spatial_pan_state)
                                {
                                    let panned_source =
                                        MonoToPanned::new(jitter_buffer, pan_state.clone());
                                    spatial_sink.append(panned_source);
                                }
                                bundle.spatial_handle = Some(handle.clone());
                            }
                            Err(e) => {
                                warn!(
                                    "Failed to create spatial jitter buffer for {}: {:?}",
                                    sink_key, e
                                );
                                continue;
                            }
                        }
                    } else if let Some(handle) = &bundle.spatial_handle {
                        let _ = handle.enqueue(packet.clone());
                    }
                } else {
                    if bundle.normal.is_none() {
                        let rodio_sink = Arc::new(Player::connect_new(&mixer));
                        let sink = Arc::new(AudioSink::Normal(rodio_sink));
                        sink.play();
                        bundle.normal = Some(sink);
                    }

                    if let Some(normal_sink) = &bundle.normal {
                        let mute_mult = if global_mute.load(Ordering::Relaxed) {
                            0.0
                        } else {
                            1.0
                        };
                        let volume =
                            1.3 * PerceptualGain::amplitude(gain_settings.gain) * mute_mult;
                        normal_sink.set_volume(volume);
                    }

                    if bundle.normal_handle.is_none() {
                        let stats = Arc::new(PlayerReceiveStats::new(display_name.clone()));
                        peer_registry.register(
                            sink_key.clone(),
                            PeerRoute::Normal,
                            stats.clone(),
                        );
                        match JitterBuffer::create_with_handle_and_activity(
                            packet.clone(),
                            format!("normal_{}", sink_key),
                            identity.clone(),
                            activity_tx.clone(),
                            recording_producer.clone(),
                            recording_active.clone(),
                            stats,
                            transport,
                        ) {
                            Ok((jitter_buffer, handle)) => {
                                if let Some(normal_sink) = &bundle.normal {
                                    let stereo_source = MonoToStereo::new(jitter_buffer);
                                    normal_sink.append(stereo_source);
                                }
                                bundle.normal_handle = Some(handle.clone());
                            }
                            Err(e) => {
                                warn!(
                                    "Failed to create normal jitter buffer for {}: {:?}",
                                    sink_key, e
                                );
                                continue;
                            }
                        }
                    } else if let Some(handle) = &bundle.normal_handle {
                        let _ = handle.enqueue(packet.clone());
                    }
                }

                sinks.insert(sink_key.clone(), bundle);
            }
        });

        self.sweep_handle = Some(Self::spawn_retirement_sweep(
            self.sinks.clone(),
            self.peer_registry.clone(),
            self.shutdown.clone(),
        ));

        Ok(handle)
    }

    pub async fn stop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);

        // Aborted rather than left to notice the flag: it sleeps for whole seconds between
        // passes, and a torn-down mixer must not have a sink retired out from under it.
        if let Some(sweep) = self.sweep_handle.take() {
            sweep.abort();
        }

        for (_, bundle) in self.sinks.iter() {
            if let Some(h) = &bundle.normal_handle {
                h.stop();
            }
            if let Some(h) = &bundle.spatial_handle {
                h.stop();
            }
            if let Some(s) = &bundle.normal {
                s.clear_and_stop();
            }
            if let Some(s) = &bundle.spatial {
                s.clear_and_stop();
            }
        }

        info!("SinkManager has been stopped.");
    }
}
