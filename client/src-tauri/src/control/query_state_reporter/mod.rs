use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use common::structs::audio::PlayerGainStore;
use common::structs::control::{BvcsCodec, PlayerPreference, QueryState};
use common::structs::packet::{
    PacketType, PlayerPreferencePacket, QueryStatePacket, QuicNetworkPacket, QuicNetworkPacketData,
};
use log::{debug, warn};
use tauri::Manager;
use tokio::sync::broadcast;

use super::connection_identity::ConnectionIdentity;
use super::state_signal::ControlStateSignal;
use crate::NetworkPacket;
use crate::audio::AudioActionsManager;
use crate::players::PlayerSettingsCoordinator;
#[cfg(feature = "bedrock-protocol")]
use crate::bedrock::QueryStateInjector;

// Coalesce bursts of state changes into at most one report wave per window
// (~5 waves/second), so slider drags and rapid toggles don't flood the server.
const DEBOUNCE_WINDOW: Duration = Duration::from_millis(200);

// Event-driven reports alone cannot heal state the receiver lost: a server
// restart wipes the control caches, QueryState/PlayerPreference ride lossy
// QUIC datagrams, and a BDS reload drops its panel caches. Every interval the
// full state re-pushes unconditionally (the preference diff is cleared so
// unchanged entries go out again), bounding any de-sync to one period.
const RESYNC_INTERVAL: Duration = Duration::from_secs(30);

/// Pushes the client's live audio-control state to the server and the no-net
/// panel: on every `ControlStateSignal` it assembles the self-state
/// (mute/deafen/record) and/or the changed per-player preferences from the
/// persisted `player_gain_store`, then sends them two ways —
///
/// * ServerBound `QueryState` / `PlayerPreference` QUIC packets, keeping the
///   server's control cache fresh for the net panel's `/api/state` poll. `id`/`owner`
///   come from `ConnectionIdentity`, which holds the canonical identity the server
///   authenticated this connection as; the server's authorship guard compares against
///   exactly that and drops anything else.
/// * Encoded `!bvcs:` rides into the `QueryStateInjector`, which the proxy
///   session injects as serverbound chat for the no-net panel — this path needs
///   no identity and keeps working while the QUIC link is down.
pub struct QueryStateReporter {
    app_handle: tauri::AppHandle,
    // Last-sent per-player preference values; only changed entries are re-sent,
    // which also bounds the on-connect burst to the store's actual contents.
    last_prefs: HashMap<String, (f32, bool)>,
    // Identity the last QUIC report was sent as; a change (new connection)
    // resets the diff so the new server receives every preference once.
    last_identity: Option<String>,
    // Monotonic tag on every !bvcs: message.
    seq: u64,
}

impl QueryStateReporter {
    pub fn new(app_handle: tauri::AppHandle) -> Self {
        Self {
            app_handle,
            last_prefs: HashMap::new(),
            last_identity: None,
            seq: 0,
        }
    }

    /// Consume the control-state bus until every sender is dropped, coalescing
    /// signal bursts within `DEBOUNCE_WINDOW` into a single report wave and
    /// re-pushing the full state every `RESYNC_INTERVAL` regardless of events.
    pub async fn run(mut self, mut rx: broadcast::Receiver<ControlStateSignal>) {
        let mut resync = tokio::time::interval(RESYNC_INTERVAL);
        resync.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        // An interval's first tick resolves immediately; consume it so startup
        // does not report before anything is connected.
        resync.tick().await;
        loop {
            let mut wave = ReportWave::default();
            tokio::select! {
                received = rx.recv() => match received {
                    Ok(signal) => wave.fold(signal),
                    // Missed signals could have been either kind; send everything.
                    Err(broadcast::error::RecvError::Lagged(_)) => wave.fold_all(),
                    Err(broadcast::error::RecvError::Closed) => return,
                },
                _ = resync.tick() => {
                    // Full re-push: clearing the diff forces every preference
                    // out again, not just changed ones.
                    self.last_prefs.clear();
                    wave.fold_all();
                }
            }

            let deadline = tokio::time::Instant::now() + DEBOUNCE_WINDOW;
            loop {
                match tokio::time::timeout_at(deadline, rx.recv()).await {
                    Ok(Ok(signal)) => wave.fold(signal),
                    Ok(Err(broadcast::error::RecvError::Lagged(_))) => wave.fold_all(),
                    Ok(Err(broadcast::error::RecvError::Closed)) => {
                        self.report(wave).await;
                        return;
                    }
                    Err(_elapsed) => break,
                }
            }

            self.report(wave).await;
        }
    }

    async fn report(&mut self, wave: ReportWave) {
        let id = {
            let identity = self.app_handle.state::<Arc<ConnectionIdentity>>();
            identity.get()
        };
        if self.last_identity != id {
            self.last_prefs.clear();
            self.last_identity = id.clone();
        }

        // The panel's sync request always warrants a fresh self-state ride.
        if wave.self_state || wave.sync {
            let actions = AudioActionsManager::new(self.app_handle.clone());
            let s = actions.query_state().await;
            let state = QueryState {
                id: id.clone().unwrap_or_default(),
                muted: s.muted,
                deafened: s.deafened,
                recording: s.recording,
                // Server-authoritative: overlaid from channel membership at read.
                current_group: None,
            };

            if wave.self_state {
                if let Some(id) = &id {
                    let state = QueryState {
                        id: id.clone(),
                        ..state.clone()
                    };
                    self.send_quic(
                        PacketType::QueryState,
                        QuicNetworkPacketData::QueryState(QueryStatePacket::new(state)),
                    );
                } else {
                    debug!("QueryStateReporter: no connection identity; QUIC report skipped");
                }
            }
            let seq = self.next_seq();
            self.ride_bvcs(BvcsCodec::encode_query_state(seq, &state));
        }

        if wave.preferences || !wave.sync_targets.is_empty() {
            // Read from the settings service rather than `store.json`, which no longer
            // carries these. Scoped to the current server by `store_for`, which is correct
            // here for the same reason it is at the mixer: a preference is reported to the
            // server it was set on.
            let gains: PlayerGainStore = match self
                .app_handle
                .try_state::<Arc<PlayerSettingsCoordinator>>()
            {
                Some(coordinator) => coordinator
                    .store_for_current_server(&self.app_handle)
                    .await
                    .unwrap_or_default(),
                None => PlayerGainStore::default(),
            };

            // The jukebox rides this plane as a reserved target, so it is one more entry in the
            // same iteration rather than a second reporting path — the diff, the QUIC report and
            // the !bvcs: ride below all apply to it unchanged. Synthesised here and never
            // persisted: an entry in the gain store would render as a player card.
            //
            // Read after the store, never before: the coordinator read above takes the AppState
            // lock and these take the audio stream lock, which is the order `set_audio_device`
            // already establishes. Reversing it would complete a deadlock cycle.
            let jukebox = {
                let actions = AudioActionsManager::new(self.app_handle.clone());
                (
                    common::consts::audio::JUKEBOX_CONTROL_TARGET.to_string(),
                    actions.jukebox_gain().await,
                    actions.jukebox_muted().await,
                )
            };
            let entries: Vec<(String, f32, bool)> = gains
                .0
                .iter()
                .map(|(target, settings)| (target.clone(), settings.gain, settings.muted))
                .chain(std::iter::once(jukebox))
                .collect();

            for (target, gain, muted) in entries.iter() {
                let entry = (*gain, *muted);
                let changed = self.last_prefs.get(target) != Some(&entry);
                let synced = wave.sync_targets.contains(target);
                if !(wave.preferences && changed) && !synced {
                    continue;
                }

                let preference = PlayerPreference {
                    owner: id.clone().unwrap_or_default(),
                    target: target.clone(),
                    volume: *gain,
                    muted: *muted,
                };

                if wave.preferences && changed {
                    self.last_prefs.insert(target.clone(), entry);
                    if let Some(id) = &id {
                        let preference = PlayerPreference {
                            owner: id.clone(),
                            ..preference.clone()
                        };
                        self.send_quic(
                            PacketType::PlayerPreference,
                            QuicNetworkPacketData::PlayerPreference(PlayerPreferencePacket::new(
                                preference,
                            )),
                        );
                    }
                }
                // A delimiter-bearing target would corrupt the text grammar; the
                // binary QUIC report above is unaffected.
                if BvcsCodec::target_is_wire_safe(target) {
                    let seq = self.next_seq();
                    self.ride_bvcs(BvcsCodec::encode_preference(seq, &preference));
                } else {
                    debug!("QueryStateReporter: target not !bvcs:-safe; ride skipped: {target}");
                }
            }
        }
    }

    fn next_seq(&mut self) -> u64 {
        self.seq = self.seq.wrapping_add(1);
        self.seq
    }

    fn send_quic(&self, packet_type: PacketType, data: QuicNetworkPacketData) {
        let producer = self
            .app_handle
            .state::<Arc<flume::Sender<NetworkPacket>>>();
        let packet = NetworkPacket {
            data: QuicNetworkPacket {
                packet_type,
                // Carries no sender: the server stamps one from the certificate at ingress.
                data,
                            // Not a server fan-out, so this envelope carries no sequence.
                ..Default::default()
            },
        };
        if let Err(e) = producer.try_send(packet) {
            warn!("QueryStateReporter: dropping state report: {e}");
        }
    }

    // Without the bedrock proxy there is no no-net panel to ride state into.
    #[cfg(feature = "bedrock-protocol")]
    fn ride_bvcs(&self, message: String) {
        if let Some(injector) = self.app_handle.try_state::<Arc<QueryStateInjector>>() {
            injector.enqueue(message);
        }
    }

    #[cfg(not(feature = "bedrock-protocol"))]
    fn ride_bvcs(&self, _message: String) {}
}

mod report_wave;

use report_wave::ReportWave;
