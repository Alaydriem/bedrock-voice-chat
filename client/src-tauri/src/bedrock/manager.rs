use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use bytes::BytesMut;
use common::bedrock_protocol::{
    AuthInfo, AuthManager, Bytes, DisconnectPacket, Proxy, ProxyConfig,
    RealmConfig, Session,
    proxy::{WarmPool, WarmTarget},
    protocol::batch::BatchCodec,
    protocol::codec::PacketEncode,
    protocol::packets::{PacketHeader, ids},
    protocol::packets::generated::misc::text::TextPacket,
    protocol::types::generated::{AuthorAndMessage, TextPacketBody, TextPacketType},
};
use rand::RngExt;
use common::structs::{AnalyticsEvent, AnalyticsEventData};
use common::traits::StreamTrait;
use log::{error, info, warn};
use tokio::sync::{oneshot, watch};
use tokio::task::{AbortHandle, JoinHandle};

use crate::bedrock::event_emitter::BedrockEventEmitter;
use crate::bedrock::jukebox_beacon_cache::JukeboxBeaconCache;
use crate::bedrock::jukebox_eject_injector::JukeboxEjectInjector;
use crate::bedrock::pending_eject::PendingEject;
use crate::bedrock::session_event::{BedrockSessionEventDispatcher, DispatchOutcome};

const RELAY_DRAIN_DELAY: Duration = Duration::from_millis(500);

const CLIENT_DISCONNECT_DRAIN: Duration = Duration::from_millis(150);

const POSITION_HEARTBEAT_INTERVAL: Duration = Duration::from_millis(250);

const BVC_DISCONNECT_MESSAGE: &str =
    "Connection was closed by the Bedrock Voice Chat app so your link to this server was ended on purpose. Reconnect through the Bedrock Voice Chat app reconnect to this server.";

use crate::bedrock::backend::Backend;
use crate::bedrock::connect_error_channel::{self, BedrockConnectErrorChannel};
use crate::bedrock::player_state_cache::BedrockPlayerStateCache;
use crate::bedrock::services::ProtocolGatingService;
use crate::bedrock::session_state::BedrockSessionState;

pub struct BedrockProxyManager {
    listen_port: u16,
    backend: Option<Backend>,
    player_state_cache: Arc<BedrockPlayerStateCache>,
    event_emitter: Option<Arc<BedrockEventEmitter>>,
    gating: Arc<ProtocolGatingService>,
    beacon_cache: Arc<JukeboxBeaconCache>,
    eject_injector: Option<Arc<JukeboxEjectInjector>>,
    error_channel: Arc<BedrockConnectErrorChannel>,
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
        beacon_cache: Arc<JukeboxBeaconCache>,
        error_channel: Arc<BedrockConnectErrorChannel>,
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
            beacon_cache,
            eject_injector: None,
            error_channel,
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
        beacon_cache: Arc<JukeboxBeaconCache>,
        error_channel: Arc<BedrockConnectErrorChannel>,
    ) -> Self {
        let auth = AuthInfo::xbl_token_with_access(xbl_token, user_hash, access_token);
        let realm_config = RealmConfig::new(realm_id, realms_api);
        Self {
            listen_port,
            backend: Some(Backend::Realm { realm_config, auth }),
            player_state_cache,
            event_emitter: None,
            gating,
            beacon_cache,
            eject_injector: None,
            error_channel,
            jobs: vec![],
            shutdown: Arc::new(AtomicBool::new(false)),
            shutdown_tx: None,
            listener_handle: None,
        }
    }

    pub fn set_event_emitter(&mut self, emitter: Arc<BedrockEventEmitter>) {
        self.event_emitter = Some(emitter);
    }

    pub fn set_eject_injector(&mut self, injector: Arc<JukeboxEjectInjector>) {
        self.eject_injector = Some(injector);
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
        let beacon_cache = Arc::clone(&self.beacon_cache);
        let eject_injector = self.eject_injector.clone();
        let error_channel = Arc::clone(&self.error_channel);

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
                        let child_beacon_cache = Arc::clone(&beacon_cache);
                        let child_eject_rx = eject_injector.as_ref().map(|i| i.receiver());
                        let child_error_channel = Arc::clone(&error_channel);
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
                                            child_error_channel.emit(
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
                                    child_error_channel.emit(
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

                            let mut dispatcher = BedrockSessionEventDispatcher::new(
                                child_player_name.clone(),
                                Arc::clone(&child_beacon_cache),
                                Arc::clone(&child_cache),
                                child_emitter.clone(),
                            );

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
                                        match dispatcher.dispatch(&evt, &mut state) {
                                            DispatchOutcome::Continue => {}
                                            DispatchOutcome::SessionEnded { reason, detail } => {
                                                disconnect_reason = reason;
                                                disconnect_detail = detail;
                                                break;
                                            }
                                        }
                                    }
                                    eject = async {
                                        match child_eject_rx.as_ref() {
                                            Some(rx) => rx.recv_async().await.ok(),
                                            None => std::future::pending::<Option<PendingEject>>().await,
                                        }
                                    } => {
                                        if let Some(eject) = eject {
                                            Self::dispatch_jukebox_eject(
                                                &session,
                                                &state,
                                                &child_player_name,
                                                eject,
                                            );
                                        }
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

    fn dispatch_jukebox_eject(
        session: &Session,
        state: &BedrockSessionState,
        player_name: &str,
        eject: PendingEject,
    ) {
        let local_world = state.world_uuid().unwrap_or("");
        if local_world != eject.world_uuid {
            info!(
                "Bedrock: dropping jukebox eject (world mismatch local={} ann={}) event_id={}",
                local_world, eject.world_uuid, eject.event_id
            );
            return;
        }

        let writer = session.writer().clone();
        let version = session.protocol_version();
        let player_xuid = session.player.xuid.clone();
        let chat_sender_name = session.player.name.clone();
        let player_name_owned = player_name.to_string();
        let event_id = eject.event_id.clone();
        info!(
            "Bedrock: dispatching jukebox eject event_id={} pos=({},{},{}) world={}",
            event_id,
            eject.block_pos.x.floor() as i32,
            eject.block_pos.y.floor() as i32,
            eject.block_pos.z.floor() as i32,
            eject.world_uuid
        );

        tokio::spawn(async move {
            let stagger_ms = rand::rng().random_range(0u64..500u64);
            tokio::time::sleep(Duration::from_millis(stagger_ms)).await;

            let message = format!(
                "!bvce {} {} {}",
                eject.block_pos.x.floor() as i32,
                eject.block_pos.y.floor() as i32,
                eject.block_pos.z.floor() as i32,
            );

            let text = TextPacket {
                localize: false,
                body: TextPacketBody::AuthorAndMessage(AuthorAndMessage {
                    message_type: TextPacketType::Chat,
                    player_name: chat_sender_name,
                    message: message.clone(),
                }),
                sender_s_xuid: player_xuid,
                platform_id: String::new(),
                filtered_message: None,
            };

            let mut pkt_buf = BytesMut::new();
            PacketHeader::write(&mut pkt_buf, ids::TEXT);
            text.encode_for(version, &mut pkt_buf);

            let batch: Bytes = match BatchCodec::encode(&[pkt_buf.freeze()], true, 1) {
                Ok(b) => b,
                Err(e) => {
                    warn!(
                        "Bedrock: failed to batch-encode chat for jukebox eject event_id={}: {:?}",
                        event_id, e
                    );
                    return;
                }
            };

            let batch_len = batch.len();
            let batch_preview: Vec<u8> = batch.iter().take(32).copied().collect();
            match writer.send_to_server(batch) {
                Ok(()) => debug!(
                    "Bedrock: injected silent chat '{}' event_id={} (stagger {}ms, batch_len={}, head={:02X?})",
                    message, event_id, stagger_ms, batch_len, batch_preview
                ),
                Err(e) => warn!(
                    "Bedrock: failed to inject silent chat for jukebox eject (player {}) event_id={}: {:?}",
                    player_name_owned, event_id, e
                ),
            }
        });
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
        let beacon = Arc::new(JukeboxBeaconCache::new());
        let error_channel = Arc::new(BedrockConnectErrorChannel::new());
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
            Arc::clone(&beacon),
            Arc::clone(&error_channel),
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
            beacon,
            error_channel,
        );
        mgr2.start().await.expect("second start on same port");
        tokio::time::sleep(Duration::from_secs(2)).await;
        mgr2.stop().await.expect("second stop");
    }
}
