use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;
use std::time::Duration;

use common::structs::audio::PlayerGainStore;
use common::structs::control::{BvcsCodec, PlayerPreference, QueryState};
use common::structs::packet::{
    PacketType, PlayerPreferencePacket, QueryStatePacket, QuicNetworkPacket, QuicNetworkPacketData,
};
use log::{debug, warn};
use tauri::Manager;
use tauri_plugin_store::StoreExt;
use tokio::sync::broadcast;

use super::connection_identity::ConnectionIdentity;
use super::state_signal::ControlStateSignal;
use crate::NetworkPacket;
use crate::audio::AudioActionsManager;
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
///   server's control cache fresh for the net panel's `/api/state` poll. The
///   network `OutputStream` stamps the packet owner, so `id`/`owner` come from
///   `ConnectionIdentity` (the same name the stamp uses) or the server's
///   authorship guard would drop the report.
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
            let gains: PlayerGainStore = self
                .app_handle
                .store("store.json")
                .ok()
                .and_then(|store| store.get("player_gain_store"))
                .and_then(|v| serde_json::from_value(v).ok())
                .unwrap_or_default();

            for (target, settings) in gains.0.iter() {
                let entry = (settings.gain, settings.muted);
                let changed = self.last_prefs.get(target) != Some(&entry);
                let synced = wave.sync_targets.contains(target);
                if !(wave.preferences && changed) && !synced {
                    continue;
                }

                let preference = PlayerPreference {
                    owner: id.clone().unwrap_or_default(),
                    target: target.clone(),
                    volume: settings.gain,
                    muted: settings.muted,
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
                // Stamped with the connection identity by the network OutputStream.
                owner: None,
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

// One debounced wave of coalesced signals.
#[derive(Default)]
struct ReportWave {
    self_state: bool,
    preferences: bool,
    sync: bool,
    sync_targets: BTreeSet<String>,
}

impl ReportWave {
    fn fold(&mut self, signal: ControlStateSignal) {
        match signal {
            ControlStateSignal::SelfState => self.self_state = true,
            ControlStateSignal::Preferences => self.preferences = true,
            ControlStateSignal::Sync { targets } => {
                self.sync = true;
                self.sync_targets.extend(targets);
            }
        }
    }

    fn fold_all(&mut self) {
        self.self_state = true;
        self.preferences = true;
    }
}
