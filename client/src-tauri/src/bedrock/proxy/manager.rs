use std::net::SocketAddr;
use std::sync::Arc;

use bedrock_protocol::{
    AuthManager, Direction, PlayerAuthInputPacket,
    Proxy, ProxyConfig,
};
use common::players::minecraft::MinecraftPlayer;
use common::players::PlayerEnum;
use common::structs::game::coordinate::Coordinate;
use common::structs::game::orientation::Orientation;
use common::traits::StreamTrait;
use tokio::sync::watch;

use crate::bedrock::position_cache::BedrockPositionCache;

pub struct ProxyConnectManager {
    target_host: String,
    target_port: u16,
    listen_port: u16,
    auth_manager: Arc<AuthManager>,
    position_cache: Arc<BedrockPositionCache>,
    shutdown_tx: Option<watch::Sender<bool>>,
    runtime_thread: Option<std::thread::JoinHandle<()>>,
}

impl ProxyConnectManager {
    pub fn new(
        target_host: String,
        target_port: u16,
        listen_port: u16,
        auth_manager: Arc<AuthManager>,
        position_cache: Arc<BedrockPositionCache>,
    ) -> Self {
        Self {
            target_host,
            target_port,
            listen_port,
            auth_manager,
            position_cache,
            shutdown_tx: None,
            runtime_thread: None,
        }
    }
}

impl ProxyConnectManager {
    async fn run_loop(
        listen_port: u16,
        target_addr: SocketAddr,
        auth_manager: Arc<AuthManager>,
        position_cache: Arc<BedrockPositionCache>,
        mut shutdown_rx: watch::Receiver<bool>,
        ready_tx: tokio::sync::oneshot::Sender<Result<(), String>>,
    ) {
        let config = ProxyConfig {
            bind: match format!("0.0.0.0:{}", listen_port).parse() {
                Ok(addr) => addr,
                Err(e) => {
                    let _ = ready_tx.send(Err(e.to_string()));
                    return;
                }
            },
            motd: "BVC Proxy".to_string(),
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

        log::info!("Bedrock proxy listening on {}", proxy.local_addr());
        let _ = ready_tx.send(Ok(()));

        loop {
            tokio::select! {
                conn = proxy.accept() => {
                    let Some(conn) = conn else {
                        break;
                    };

                    let player_name = conn.player.name.clone();
                    log::info!("Proxy: player connected: {}", player_name);

                    position_cache.set_local_gamertag(player_name.clone());

                    let packet_cache = Arc::clone(&position_cache);
                    let conn = conn
                        .on_packet::<PlayerAuthInputPacket, _>(move |direction, packet: &mut PlayerAuthInputPacket| {
                            if matches!(direction, Direction::Serverbound) {
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
                                packet_cache.set(&player_name, player);
                            }
                        });

                    let auth_mgr = Arc::clone(&auth_manager);
                    tokio::spawn(async move {
                        let auth = match auth_mgr.auth_for(&conn.player, |code, url, _name| {
                            log::info!("Device code: {} URL: {}", code, url);
                        }).await {
                            Ok(auth) => auth,
                            Err(e) => {
                                log::error!("Auth failed for proxy: {}", e);
                                return;
                            }
                        };

                        match conn.connect_to(target_addr, auth, None::<fn(&str, &str)>).await {
                            Ok(mut session) => {
                                log::info!("Proxy session started");
                                while let Some(event) = session.next().await {
                                    match event {
                                        bedrock_protocol::Event::Disconnected(reason) => {
                                            log::info!("Proxy session disconnected: {}", reason);
                                            break;
                                        }
                                        _ => {}
                                    }
                                }
                                log::info!("Proxy session ended");
                            }
                            Err(e) => {
                                log::error!("Proxy connection failed: {}", e);
                            }
                        }
                    });
                }
                _ = shutdown_rx.wait_for(|&v| v) => {
                    log::info!("Proxy shutdown signal received");
                    break;
                }
            }
        }
    }
}

impl StreamTrait for ProxyConnectManager {
    async fn start(&mut self) -> Result<(), anyhow::Error> {
        if self.runtime_thread.is_some() {
            return Err(anyhow::anyhow!("Proxy is already running"));
        }

        let target_addr: SocketAddr = tokio::net::lookup_host(
            format!("{}:{}", self.target_host, self.target_port),
        )
        .await?
        .next()
        .ok_or_else(|| anyhow::anyhow!(
            "Failed to resolve host: {}",
            self.target_host,
        ))?;

        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        self.shutdown_tx = Some(shutdown_tx);

        let (ready_tx, ready_rx) = tokio::sync::oneshot::channel::<Result<(), String>>();

        let listen_port = self.listen_port;
        let auth_manager = Arc::clone(&self.auth_manager);
        let position_cache = Arc::clone(&self.position_cache);

        let thread = std::thread::spawn(move || {
            let rt = match tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
            {
                Ok(rt) => rt,
                Err(e) => {
                    let _ = ready_tx.send(Err(e.to_string()));
                    return;
                }
            };

            rt.block_on(async move {
                Self::run_loop(
                    listen_port,
                    target_addr,
                    auth_manager,
                    position_cache,
                    shutdown_rx,
                    ready_tx,
                ).await;
            });

            log::info!("Proxy runtime shut down, all sockets released");
        });

        self.runtime_thread = Some(thread);

        ready_rx.await
            .map_err(|_| anyhow::anyhow!("Proxy thread died before signalling ready"))?
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    async fn stop(&mut self) -> Result<(), anyhow::Error> {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(true);
        }

        if let Some(thread) = self.runtime_thread.take() {
            let _ = tokio::task::spawn_blocking(move || {
                let _ = thread.join();
            }).await;
        }

        self.position_cache.clear();
        log::info!("Proxy connect manager stopped");
        Ok(())
    }

    fn is_stopped(&self) -> bool {
        self.runtime_thread.is_none()
    }

    async fn metadata(&mut self, _key: String, _value: String) -> Result<(), anyhow::Error> {
        Ok(())
    }
}
