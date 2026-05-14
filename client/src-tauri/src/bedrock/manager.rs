use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use bytes::BytesMut;
use common::bedrock_protocol::{
    AuthInfo, AuthManager, Bytes, Direction, DisconnectPacket, Event, Proxy, ProxyConfig,
    RealmConfig, Session,
    proxy::{WarmPool, WarmTarget},
    protocol::batch::BatchCodec,
    protocol::codec::PacketEncode,
    protocol::packets::{PacketHeader, ids},
    protocol::types::transaction::TransactionData,
    protocol::types::use_item_action_type::UseItemActionType,
};
use common::structs::game::Coordinate;
use common::structs::packet::BedrockEvent;
use common::structs::{AnalyticsEvent, AnalyticsEventData};
use common::traits::StreamTrait;
use log::{debug, error, info, warn};
use tokio::sync::{oneshot, watch};
use tokio::task::{AbortHandle, JoinHandle};

use crate::bedrock::bvc_disc_nbt::BvcDiscNbt;
use crate::bedrock::event_emitter::BedrockEventEmitter;
use crate::bedrock::jukebox_beacon_cache::JukeboxBeaconCache;

const RELAY_DRAIN_DELAY: Duration = Duration::from_millis(500);

const CLIENT_DISCONNECT_DRAIN: Duration = Duration::from_millis(150);

const POSITION_HEARTBEAT_INTERVAL: Duration = Duration::from_millis(250);

const BVC_DISCONNECT_MESSAGE: &str =
    "Connection was closed by the Bedrock Voice Chat app so your link to this server was ended on purpose. Reconnect through the Bedrock Voice Chat app reconnect to this server.";

use crate::bedrock::backend::Backend;
use crate::bedrock::connect_error_channel;
use crate::bedrock::player_state_cache::BedrockPlayerStateCache;
use crate::bedrock::services::ProtocolGatingService;
use crate::bedrock::session_state::BedrockSessionState;

pub struct BedrockProxyManager {
    listen_port: u16,
    backend: Option<Backend>,
    player_state_cache: Arc<BedrockPlayerStateCache>,
    event_emitter: Option<Arc<BedrockEventEmitter>>,
    gating: Arc<ProtocolGatingService>,
    jobs: Vec<AbortHandle>,
    shutdown: Arc<AtomicBool>,
    shutdown_tx: Option<oneshot::Sender<()>>,
    listener_handle: Option<JoinHandle<()>>,
}

impl BedrockProxyManager {
    pub fn new_direct(
        target_host: String,
        target_port: u16,
        listen_port: u16,
        auth_manager: Arc<AuthManager>,
        player_state_cache: Arc<BedrockPlayerStateCache>,
        gating: Arc<ProtocolGatingService>,
    ) -> Self {
        Self {
            listen_port,
            backend: Some(Backend::Direct {
                target_host,
                target_port,
                auth_manager,
            }),
            player_state_cache,
            event_emitter: None,
            gating,
            jobs: vec![],
            shutdown: Arc::new(AtomicBool::new(false)),
            shutdown_tx: None,
            listener_handle: None,
        }
    }

    pub fn new_realm(
        realm_id: u64,
        listen_port: u16,
        xbl_token: String,
        user_hash: String,
        access_token: String,
        realms_api: common::bedrock_protocol::RealmsApi,
        player_state_cache: Arc<BedrockPlayerStateCache>,
        gating: Arc<ProtocolGatingService>,
    ) -> Self {
        let auth = AuthInfo::xbl_token_with_access(xbl_token, user_hash, access_token);
        let realm_config = RealmConfig::new(realm_id, realms_api);
        Self {
            listen_port,
            backend: Some(Backend::Realm { realm_config, auth }),
            player_state_cache,
            event_emitter: None,
            gating,
            jobs: vec![],
            shutdown: Arc::new(AtomicBool::new(false)),
            shutdown_tx: None,
            listener_handle: None,
        }
    }

    pub fn set_event_emitter(&mut self, emitter: Arc<BedrockEventEmitter>) {
        self.event_emitter = Some(emitter);
    }
}

impl StreamTrait for BedrockProxyManager {
    async fn start(&mut self) -> Result<(), anyhow::Error> {
        if !self.jobs.is_empty() {
            return Err(anyhow::anyhow!("Bedrock manager is already running"));
        }
        let _ = self.shutdown.store(false, Ordering::Relaxed);

        let mut jobs = vec![];

        match self.listener(self.shutdown.clone()) {
            Ok(job) => jobs.push(job),
            Err(e) => {
                error!("Bedrock listener encountered an error: {:?}", e);
                return Err(e);
            }
        };

        self.jobs = jobs.iter().map(|h| h.abort_handle()).collect();
        if let Some(handle) = jobs.into_iter().next() {
            self.listener_handle = Some(handle);
        }
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), anyhow::Error> {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        let _ = self.shutdown.store(true, Ordering::Relaxed);

        if let Some(handle) = self.listener_handle.take() {
            let _ = handle.await;
        }

        self.jobs = vec![];
        self.player_state_cache.clear();
        info!("Bedrock connect manager stopped");
        Ok(())
    }

    fn is_stopped(&self) -> bool {
        self.jobs.len() == 0
    }

    async fn metadata(&mut self, _key: String, _value: String) -> Result<(), anyhow::Error> {
        Ok(())
    }
}

impl BedrockProxyManager {
    fn listener(&mut self, _shutdown: Arc<AtomicBool>) -> Result<JoinHandle<()>, anyhow::Error> {
        let backend = self
            .backend
            .take()
            .ok_or_else(|| anyhow::anyhow!("Backend missing — manager already started once"))?;

        let (shutdown_tx, mut shutdown_rx) = oneshot::channel::<()>();
        self.shutdown_tx = Some(shutdown_tx);

        let listen_port = self.listen_port;
        let player_state_cache = Arc::clone(&self.player_state_cache);
        let event_emitter = self.event_emitter.clone();
        let gating = Arc::clone(&self.gating);

        let handle = tokio::spawn(async move {
            let bind_addr: SocketAddr = match format!("0.0.0.0:{}", listen_port).parse() {
                Ok(a) => a,
                Err(e) => {
                    error!("Bedrock listener invalid bind: {}", e);
                    return;
                }
            };

            let (proxy_config_realm, motd, sub_motd) = match &backend {
                Backend::Direct { .. } => (
                    None,
                    "BVC Proxy Connect".to_string(),
                    "Bedrock Voice Chat Proxy".to_string(),
                ),
                Backend::Realm { realm_config, .. } => (
                    Some(realm_config.clone()),
                    "BVC Realms Connect".to_string(),
                    "Bedrock Voice Chat Realms Proxy".to_string(),
                ),
            };

            let config = ProxyConfig {
                bind: bind_addr,
                realm: proxy_config_realm,
                motd,
                sub_motd,
                fail_disconnect_message: Some(
                    "BVC could not connect to the upstream server. \
                     Check the BVC client for details, then try again.".to_string(),
                ),
                ..Default::default()
            };

            let mut proxy = match Proxy::new(config).await {
                Ok(p) => p,
                Err(e) => {
                    error!("Bedrock proxy bind failed: {}", e);
                    return;
                }
            };
            info!("Bedrock proxy listening on {}", proxy.local_addr());

            // Fire backend-specific lifecycle event after the listener has
            // actually bound. The Direct vs Realm split mirrors the
            // user-facing distinction in the BVC UI.
            let listening_addr = proxy.local_addr().to_string();
            match &backend {
                Backend::Direct {
                    target_host,
                    target_port,
                    ..
                } => {
                    let data = AnalyticsEventData::new()
                        .insert("listen_addr", listening_addr.clone())
                        .insert("target_host", target_host.clone())
                        .insert("target_port", *target_port as i64);
                    gating.analytics().track(
                        AnalyticsEvent::BedrockProxyStarted,
                        Some(data),
                    );
                }
                Backend::Realm { .. } => {
                    let data = AnalyticsEventData::new()
                        .insert("listen_addr", listening_addr.clone());
                    gating.analytics().track(
                        AnalyticsEvent::BedrockRealmStarted,
                        Some(data),
                    );
                }
            }

            // Resolve the direct backend hostname once. Realm backends don't need this —
            // the Realms API resolves the live world address inside dial_realm.
            let direct_target_addr: Option<SocketAddr> = match &backend {
                Backend::Direct { target_host, target_port, .. } => {
                    match tokio::net::lookup_host(format!("{}:{}", target_host, target_port)).await {
                        Ok(mut addrs) => match addrs.next() {
                            Some(addr) => Some(addr),
                            None => {
                                error!(
                                    "Bedrock listener could not resolve {}:{} — no addresses returned",
                                    target_host, target_port
                                );
                                return;
                            }
                        },
                        Err(e) => {
                            error!(
                                "Bedrock listener failed to resolve {}:{}: {}",
                                target_host, target_port, e
                            );
                            return;
                        }
                    }
                }
                Backend::Realm { .. } => None,
            };

            // WarmPool only makes sense for Direct backends. Upstream's
            // `dial_realm` requires a client GameVersion (extracted from the
            // downstream Login JWT) — at preconnect time no client has connected
            // yet, so the Realm warm dial is guaranteed to fail (warm.rs:67-73).
            // Calling it just generates a misleading "WarmPool: dial failed"
            // error on every Realm connect. Realm sessions take the lazy path
            // via `conn.connect_to_realm(...)` which has the GameVersion.
            let warm_pool: Option<Arc<WarmPool>> = match (&backend, direct_target_addr) {
                (Backend::Direct { .. }, Some(addr)) => {
                    info!("Bedrock WarmPool dialing direct backend at {}", addr);
                    Some(Arc::new(WarmPool::start(WarmTarget::Direct(addr), None)))
                }
                (Backend::Realm { .. }, _) => None,
                (Backend::Direct { .. }, None) => unreachable!("direct backend without resolved addr"),
            };

            let backend = Arc::new(backend);

            let mut child_handles: Vec<JoinHandle<()>> = vec![];
            let (child_cancel_tx, _) = watch::channel(false);

            if let Some(emitter) = event_emitter.clone() {
                let heartbeat_cache = Arc::clone(&player_state_cache);
                let mut heartbeat_cancel_rx = child_cancel_tx.subscribe();
                let heartbeat_handle = tokio::spawn(async move {
                    let mut interval = tokio::time::interval(POSITION_HEARTBEAT_INTERVAL);
                    interval.set_missed_tick_behavior(
                        tokio::time::MissedTickBehavior::Skip,
                    );
                    loop {
                        tokio::select! {
                            biased;
                            _ = heartbeat_cancel_rx.changed() => {
                                if *heartbeat_cancel_rx.borrow() {
                                    info!("Bedrock position heartbeat received cancel");
                                    break;
                                }
                            }
                            _ = interval.tick() => {
                                if let Some(player) = heartbeat_cache.get_local_player() {
                                    emitter.try_send_position(player);
                                }
                            }
                        }
                    }
                });
                child_handles.push(heartbeat_handle);
            }

            loop {
                tokio::select! {
                    _ = &mut shutdown_rx => {
                        info!("Bedrock listener received shutdown signal");
                        break;
                    }
                    conn = proxy.accept() => {
                        let Some(mut conn) = conn else { break; };
                        let player_name = conn.player.name.clone();
                        let player_uuid = if conn.player.uuid.is_empty() {
                            None
                        } else {
                            Some(conn.player.uuid.clone())
                        };
                        let peer_protocol = conn.protocol_version();

                        if !gating.is_allowed(peer_protocol).await {
                            let kick = gating.kick_message(peer_protocol);
                            warn!(
                                "Bedrock: rejecting {} on unsupported protocol {} \
                                 (not in SUPPORTED_PROTOCOLS, not flag-overridden)",
                                player_name, peer_protocol
                            );
                            conn.disconnect(&kick).await;
                            continue;
                        }

                        info!(
                            "Bedrock: player connected: {} (protocol {})",
                            player_name, peer_protocol
                        );
                        player_state_cache.set_local_gamertag(player_name.clone());

                        let backend_label = match &*backend {
                            Backend::Direct { .. } => "direct",
                            Backend::Realm { .. } => "realm",
                        };
                        let connect_data = AnalyticsEventData::new()
                            .insert("protocol", peer_protocol.0 as i64)
                            .insert("backend", backend_label);
                        gating
                            .analytics()
                            .track(AnalyticsEvent::BedrockConnected, Some(connect_data));

                        let child_cache = Arc::clone(&player_state_cache);
                        let child_warm: Option<Arc<WarmPool>> = warm_pool.clone();
                        let child_backend = Arc::clone(&backend);
                        let child_player_name = player_name.clone();
                        let child_emitter = event_emitter.clone();
                        let child_gating = Arc::clone(&gating);
                        let mut child_cancel_rx = child_cancel_tx.subscribe();

                        let h = tokio::spawn(async move {
                            let auth_for_dial = match child_backend.as_ref() {
                                Backend::Realm { auth, .. } => auth.clone(),
                                Backend::Direct { auth_manager, .. } => {
                                    match auth_manager
                                        .auth_for(&conn.player, |code, url, _name| {
                                            info!("Device code: {} URL: {}", code, url);
                                        })
                                        .await
                                    {
                                        Ok(a) => a,
                                        Err(e) => {
                                            error!("Auth failed for {}: {}", child_player_name, e);
                                            connect_error_channel::emit(
                                                common::structs::bedrock::BedrockConnectError::Auth {
                                                    message: e.to_string(),
                                                },
                                            );
                                            return;
                                        }
                                    }
                                }
                            };

                            let warm = match &child_warm {
                                Some(pool) => pool.take().await,
                                None => None,
                            };
                            let session_result = if let Some(warm) = warm {
                                info!("Bedrock: splicing {} onto warm backend", child_player_name);
                                conn.splice_onto_warm(warm, auth_for_dial, None::<fn(&str, &str)>).await
                            } else {
                                info!("Bedrock: no warm slot for {}, lazy dial", child_player_name);
                                match child_backend.as_ref() {
                                    Backend::Direct { .. } => {
                                        let addr = direct_target_addr
                                            .expect("direct backend must have resolved addr");
                                        conn.connect_to(addr, auth_for_dial, None::<fn(&str, &str)>).await
                                    }
                                    Backend::Realm { .. } => {
                                        conn.connect_to_realm(auth_for_dial, None::<fn(&str, &str)>).await
                                    }
                                }
                            };

                            let mut session = match session_result {
                                Ok(s) => s,
                                Err(e) => {
                                    error!("Bedrock connect failed for {}: {}", child_player_name, e);
                                    connect_error_channel::emit(
                                        connect_error_channel::classify(&e),
                                    );
                                    return;
                                }
                            };

                            info!("Bedrock session started for {}", child_player_name);

                            let mut state = BedrockSessionState::new(
                                child_player_name.clone(),
                                player_uuid,
                            );

                            let beacon_cache = JukeboxBeaconCache::global();
                            let mut last_known_health: Option<i32> = None;
                            let mut player_auth_input_seen = false;

                            // Captures the reason this session ended. Read after the
                            // loop to emit the BedrockDisconnected analytics event with
                            // a meaningful breakdown property.
                            let disconnect_reason: &'static str;
                            let mut disconnect_detail: Option<String> = None;

                            loop {
                                tokio::select! {
                                    biased;
                                    _ = child_cancel_rx.changed() => {
                                        if *child_cancel_rx.borrow() {
                                            info!(
                                                "Bedrock session for {} received cancel; sending Disconnect to client",
                                                child_player_name
                                            );
                                            Self::send_client_disconnect(&session, &child_player_name);
                                            tokio::time::sleep(CLIENT_DISCONNECT_DRAIN).await;
                                            disconnect_reason = "session_cancelled";
                                            break;
                                        }
                                    }
                                    evt = session.next() => {
                                        let Some(evt) = evt else {
                                            disconnect_reason = "upstream_closed";
                                            break;
                                        };
                                        match evt {
                                            Event::StartGame(_dir, packet) => state.apply_start_game(&packet),
                                            Event::PlayerAuthInput(dir, packet) => {
                                                if !player_auth_input_seen {
                                                    debug!(
                                                        "Bedrock: first PlayerAuthInput received dir={:?}",
                                                        dir
                                                    );
                                                    player_auth_input_seen = true;
                                                }
                                                state.apply_position(&packet);
                                                if matches!(dir, Direction::Serverbound) {
                                                    if let Some(tx) = &packet.transaction {
                                                        Self::on_inventory_transaction(
                                                            &tx.data,
                                                            &state,
                                                            &child_emitter,
                                                            &beacon_cache,
                                                        );
                                                    }
                                                }
                                            }
                                            Event::Interact(_dir, _packet) => continue,
                                            Event::PlayerAction(_dir, _packet) => continue,
                                            Event::ChangeDimension(_dir, packet) => state.apply_change_dimension(&packet),
                                            Event::SetPlayerGameType(_dir, packet) => state.apply_game_type(packet.gamemode),
                                            Event::UpdatePlayerGameType(_dir, packet) => state.apply_game_type(packet.gamemode),
                                            Event::InventoryTransaction(dir, packet) => {
                                                if matches!(dir, Direction::Serverbound) {
                                                    Self::on_inventory_transaction(
                                                        &packet.transaction.data,
                                                        &state,
                                                        &child_emitter,
                                                        &beacon_cache,
                                                    );
                                                }
                                                continue;
                                            }
                                            Event::UpdateBlock(dir, packet) => {
                                                if matches!(dir, Direction::Clientbound) {
                                                    Self::on_update_block(
                                                        packet.position.x,
                                                        packet.position.y,
                                                        packet.position.z,
                                                        &state,
                                                        &child_emitter,
                                                        &beacon_cache,
                                                    );
                                                }
                                                continue;
                                            }
                                            Event::SetHealth(_dir, packet) => {
                                                Self::on_set_health(
                                                    packet.health,
                                                    &mut last_known_health,
                                                    &state,
                                                    &child_emitter,
                                                );
                                                continue;
                                            }
                                            Event::Disconnected(reason) => {
                                                info!("Bedrock session disconnected for {}: {:?}", child_player_name, reason);
                                                Self::on_player_leave(&state, &child_emitter);
                                                disconnect_reason = "peer_disconnect";
                                                disconnect_detail = Some(format!("{:?}", reason));
                                                break;
                                            }
                                            _ => continue,
                                        }
                                        child_cache.set(&child_player_name, state.to_player_enum());
                                    }
                                }
                            }

                            let mut disconnect_data = AnalyticsEventData::new()
                                .insert("reason", disconnect_reason)
                                .insert("protocol", peer_protocol.0 as i64)
                                .insert("backend", backend_label);
                            if let Some(detail) = disconnect_detail {
                                disconnect_data = disconnect_data.insert("detail", detail);
                            }
                            child_gating
                                .analytics()
                                .track(AnalyticsEvent::BedrockDisconnected, Some(disconnect_data));

                            // Session drops here, which cancels the lib's relay tasks; those
                            // tasks need async time to flush their Disconnect packet to BDS.
                            drop(session);

                            info!("Bedrock session ended for {}", child_player_name);
                        });
                        child_handles.push(h);
                    }
                }
            }

            // Tell children to break first so each Session sends its
            // Disconnect packet to the downstream client and lets the
            // lib's relays flush a Disconnect to BDS. The Proxy (and its
            // shared RakNet UDP socket) MUST stay alive while this
            // happens — dropping the Proxy here would kill the listener
            // socket every client connection rides on, leaving the
            // injected Disconnect with nowhere to go.
            let _ = child_cancel_tx.send(true);

            for h in child_handles {
                let _ = h.await;
            }

            tokio::time::sleep(RELAY_DRAIN_DELAY).await;

            drop(proxy);

            info!("Bedrock accept loop drained, listener released");
        });

        Ok(handle)
    }

    fn send_client_disconnect(session: &Session, player_name: &str) {
        let pkt = DisconnectPacket {
            reason: 0,
            message_skipped: false,
            kick_message: BVC_DISCONNECT_MESSAGE.to_string(),
            filtered_message: String::new(),
        };

        let mut pkt_buf = BytesMut::new();
        PacketHeader::write(&mut pkt_buf, ids::DISCONNECT);
        pkt.encode(&mut pkt_buf);

        let batch: Bytes = match BatchCodec::encode(&[pkt_buf.freeze()], true, u16::MAX) {
            Ok(b) => b,
            Err(e) => {
                warn!("Bedrock: failed to encode Disconnect for {}: {}", player_name, e);
                return;
            }
        };

        if let Err(e) = session.writer().send_to_client(batch) {
            warn!(
                "Bedrock: failed to inject Disconnect for {} (client may have already left): {}",
                player_name, e
            );
        }
    }

    fn on_update_block(
        x: i32,
        y: i32,
        z: i32,
        state: &BedrockSessionState,
        emitter: &Option<Arc<BedrockEventEmitter>>,
        beacon_cache: &Arc<JukeboxBeaconCache>,
    ) {
        let emitter = match emitter.as_ref() {
            Some(e) => e,
            None => return,
        };

        let block_key = (x, y, z);
        let event_id = match beacon_cache.process_update_block(block_key) {
            Some(id) => id,
            None => return,
        };

        let world_uuid = match state.world_uuid() {
            Some(w) => w.to_string(),
            None => {
                debug!("Skipping JukeboxEject: UpdateBlock at cached jukebox but no world_uuid");
                return;
            }
        };

        debug!(
            "Bedrock proxy: emitting JukeboxEject event_id={} at ({},{},{})",
            event_id, x, y, z
        );
        emitter.try_send(
            BedrockEvent::JukeboxEject {
                event_id,
                player_xuid: state.player_uuid().unwrap_or("").to_string(),
            },
            world_uuid,
        );
    }

    fn on_inventory_transaction(
        data: &TransactionData,
        state: &BedrockSessionState,
        emitter: &Option<Arc<BedrockEventEmitter>>,
        beacon_cache: &Arc<JukeboxBeaconCache>,
    ) {
        let emitter = match emitter.as_ref() {
            Some(e) => e,
            None => return,
        };

        let use_item = match data {
            TransactionData::ItemUse(use_item) => use_item,
            _ => return,
        };

        if !matches!(use_item.action_type, UseItemActionType::ClickBlock) {
            return;
        }

        let world_uuid = match state.world_uuid() {
            Some(w) => w.to_string(),
            None => {
                debug!("Skipping jukebox event: no world_uuid in session state");
                return;
            }
        };

        let block_key = (
            use_item.block_position.x,
            use_item.block_position.y,
            use_item.block_position.z,
        );
        let block_pos = Coordinate {
            x: use_item.block_position.x as f32,
            y: use_item.block_position.y as f32,
            z: use_item.block_position.z as f32,
        };
        let player_xuid = state.player_uuid().unwrap_or("").to_string();

        let audio_id = match BvcDiscNbt::extract_audio_id(&use_item.held_item.extra) {
            Some(id) => id,
            None => return,
        };

        debug!(
            "Bedrock proxy: emitting JukeboxInsert audio_id={} at ({},{},{})",
            audio_id, block_key.0, block_key.1, block_key.2
        );
        beacon_cache.note_insert_pending(&block_pos);
        emitter.try_send(
            BedrockEvent::JukeboxInsert {
                audio_id,
                block_pos,
                dimension: state.dimension(),
                player_xuid,
            },
            world_uuid,
        );
    }

    fn on_set_health(
        new_health: i32,
        last_known_health: &mut Option<i32>,
        state: &BedrockSessionState,
        emitter: &Option<Arc<BedrockEventEmitter>>,
    ) {
        let previously_alive = matches!(last_known_health, Some(h) if *h > 0);
        *last_known_health = Some(new_health);

        if new_health > 0 || !previously_alive {
            return;
        }

        let emitter = match emitter.as_ref() {
            Some(e) => e,
            None => return,
        };
        let world_uuid = match state.world_uuid() {
            Some(w) => w.to_string(),
            None => return,
        };

        let event = BedrockEvent::PlayerDeath {
            player_xuid: state.player_uuid().unwrap_or("").to_string(),
            dimension: state.dimension(),
            last_pos: state.coordinates(),
        };
        info!("Bedrock proxy: emitting player death for {}", state.name());
        emitter.try_send(event, world_uuid);
    }

    fn on_player_leave(
        state: &BedrockSessionState,
        emitter: &Option<Arc<BedrockEventEmitter>>,
    ) {
        let emitter = match emitter.as_ref() {
            Some(e) => e,
            None => return,
        };
        let world_uuid = match state.world_uuid() {
            Some(w) => w.to_string(),
            None => return,
        };

        let event = BedrockEvent::PlayerLeave {
            player_xuid: state.player_uuid().unwrap_or("").to_string(),
        };
        info!("Bedrock proxy: emitting player leave for {}", state.name());
        emitter.try_send(event, world_uuid);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analytics::AnalyticsService;
    use crate::feature_flags::FeatureFlagService;
    use common::bedrock_protocol::AuthManager;

    fn build_gating() -> Arc<ProtocolGatingService> {
        let flag_service = Arc::new(FeatureFlagService::new(
            String::new(),
            String::new(),
            String::new(),
            std::time::Duration::from_secs(3600),
        ));
        let telemetry = Arc::new(crate::logging::Telemetry::new(false));
        let analytics = Arc::new(AnalyticsService::new(telemetry, String::new()));
        ProtocolGatingService::new_shared(flag_service, analytics)
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn start_stop_start_does_not_leak_port() {
        let port: u16 = 21900;
        let cache = Arc::new(BedrockPlayerStateCache::new());
        let auth_cache = moka::future::Cache::builder()
            .time_to_live(std::time::Duration::from_secs(60))
            .max_capacity(10)
            .build();
        let auth_mgr = Arc::new(AuthManager::new("0000000048183522", auth_cache));

        let mut mgr = BedrockProxyManager::new_direct(
            "127.0.0.1".into(),
            65535,
            port,
            Arc::clone(&auth_mgr),
            Arc::clone(&cache),
            build_gating(),
        );

        mgr.start().await.expect("first start");
        tokio::time::sleep(Duration::from_secs(2)).await;
        mgr.stop().await.expect("stop");
        assert!(mgr.is_stopped());

        let probe = tokio::net::UdpSocket::bind(("0.0.0.0", port))
            .await
            .expect("port still bound — manager leaked the listener");
        drop(probe);

        let mut mgr2 = BedrockProxyManager::new_direct(
            "127.0.0.1".into(),
            65535,
            port,
            auth_mgr,
            cache,
            build_gating(),
        );
        mgr2.start().await.expect("second start on same port");
        tokio::time::sleep(Duration::from_secs(2)).await;
        mgr2.stop().await.expect("second stop");
    }
}
