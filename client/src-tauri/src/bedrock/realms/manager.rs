use std::sync::Arc;

use bedrock_protocol::{
    AuthInfo, Event, Proxy, ProxyConfig, RealmConfig, RealmsApi,
};
use common::players::minecraft::MinecraftPlayer;
use common::players::PlayerEnum;
use common::structs::game::coordinate::Coordinate;
use common::structs::game::orientation::Orientation;
use common::traits::StreamTrait;
use tokio::sync::watch;
use tokio::task::JoinHandle;

use crate::bedrock::position_cache::BedrockPositionCache;

pub struct RealmsConnectManager {
    realm_id: u64,
    listen_port: u16,
    xbl_token: String,
    user_hash: String,
    access_token: String,
    realms_api: RealmsApi,
    position_cache: Arc<BedrockPositionCache>,
    shutdown_tx: Option<watch::Sender<bool>>,
    task_handle: Option<JoinHandle<()>>,
}

impl RealmsConnectManager {
    pub fn new(
        realm_id: u64,
        listen_port: u16,
        xbl_token: String,
        user_hash: String,
        access_token: String,
        realms_api: RealmsApi,
        position_cache: Arc<BedrockPositionCache>,
    ) -> Self {
        Self {
            realm_id,
            listen_port,
            xbl_token,
            user_hash,
            access_token,
            realms_api,
            position_cache,
            shutdown_tx: None,
            task_handle: None,
        }
    }
}

impl RealmsConnectManager {
    async fn run_loop(
        listen_port: u16,
        realm_id: u64,
        xbl_token: String,
        user_hash: String,
        access_token: String,
        realms_api: RealmsApi,
        position_cache: Arc<BedrockPositionCache>,
        mut shutdown_rx: watch::Receiver<bool>,
        ready_tx: tokio::sync::oneshot::Sender<Result<(), String>>,
    ) {
        let realm_config = RealmConfig::new(realm_id, realms_api);

        let config = ProxyConfig {
            bind: match format!("0.0.0.0:{}", listen_port).parse() {
                Ok(addr) => addr,
                Err(e) => {
                    let _ = ready_tx.send(Err(e.to_string()));
                    return;
                }
            },
            realm: Some(realm_config),
            motd: "BVC Realms".to_string(),
            sub_motd: "Bedrock Voice Chat".to_string(),
            ..Default::default()
        };

        let mut proxy = match Proxy::new(config).await {
            Ok(p) => p,
            Err(e) => {
                let _ = ready_tx.send(Err(e.to_string()));
                return;
            }
        };

        log::info!("Realms proxy listening on {}", proxy.local_addr());
        let _ = ready_tx.send(Ok(()));

        loop {
            tokio::select! {
                conn = proxy.accept() => {
                    let Some(conn) = conn else {
                        break;
                    };

                    let player_name = conn.player.name.clone();
                    log::info!("Realms: player connected: {}", player_name);

                    let cache = Arc::clone(&position_cache);
                    cache.set_local_gamertag(player_name.clone());

                    let auth = AuthInfo::xbl_token_with_access(
                        xbl_token.clone(),
                        user_hash.clone(),
                        access_token.clone(),
                    );

                    tokio::spawn(async move {
                        match conn.connect_to_realm(auth, None::<fn(&str, &str)>).await {
                            Ok(mut session) => {
                                log::info!("Realms session started for {}", player_name);
                                while let Some(event) = session.next().await {
                                    match event {
                                        Event::PlayerAuthInput(_dir, packet) => {
                                            let player = PlayerEnum::Minecraft(MinecraftPlayer {
                                                name: player_name.clone(),
                                                coordinates: Coordinate {
                                                    x: packet.position.x,
                                                    y: packet.position.y,
                                                    z: packet.position.z,
                                                },
                                                orientation: Orientation {
                                                    x: packet.yaw,
                                                    y: packet.pitch,
                                                },
                                                dimension: Default::default(),
                                                deafen: false,
                                                spectator: false,
                                                world_uuid: None,
                                                alternative_identity: None,
                                                player_uuid: None,
                                            });
                                            cache.set(&player_name, player);
                                        }
                                        Event::Disconnected(reason) => {
                                            log::info!("Realms session disconnected: {:?}", reason);
                                            break;
                                        }
                                        _ => {}
                                    }
                                }
                                log::info!("Realms session ended for {}", player_name);
                            }
                            Err(e) => {
                                log::error!("Realms connection failed: {}", e);
                            }
                        }
                    });
                }
                _ = shutdown_rx.wait_for(|&v| v) => {
                    log::info!("Realms shutdown signal received");
                    break;
                }
            }
        }
    }
}

impl StreamTrait for RealmsConnectManager {
    async fn start(&mut self) -> Result<(), anyhow::Error> {
        if self.task_handle.is_some() {
            return Err(anyhow::anyhow!("Realms is already running"));
        }

        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        self.shutdown_tx = Some(shutdown_tx);

        let (ready_tx, ready_rx) = tokio::sync::oneshot::channel::<Result<(), String>>();

        let listen_port = self.listen_port;
        let realm_id = self.realm_id;
        let xbl_token = self.xbl_token.clone();
        let user_hash = self.user_hash.clone();
        let access_token = self.access_token.clone();
        let realms_api = self.realms_api.clone();
        let position_cache = Arc::clone(&self.position_cache);

        let handle = tokio::spawn(async move {
            Self::run_loop(
                listen_port,
                realm_id,
                xbl_token,
                user_hash,
                access_token,
                realms_api,
                position_cache,
                shutdown_rx,
                ready_tx,
            ).await;
        });

        self.task_handle = Some(handle);

        ready_rx.await
            .map_err(|_| anyhow::anyhow!("Realms task died before signalling ready"))?
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    async fn stop(&mut self) -> Result<(), anyhow::Error> {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(true);
        }

        if let Some(handle) = self.task_handle.take() {
            handle.abort();
            let _ = handle.await;
        }

        self.position_cache.clear();
        log::info!("Realms connect manager stopped");
        Ok(())
    }

    fn is_stopped(&self) -> bool {
        self.task_handle.is_none()
    }

    async fn metadata(&mut self, _key: String, _value: String) -> Result<(), anyhow::Error> {
        Ok(())
    }
}
