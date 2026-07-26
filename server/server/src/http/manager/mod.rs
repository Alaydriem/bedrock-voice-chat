use crate::{
    config::ApplicationConfig,
    http::pool::AppDb,
    http::routes,
    services::{
        AudioPlaybackService, AudioStreamTokenCache, BedrockEventService, CertificateService,
        PlayerIdentityService, PlayerRegistrarService,
    },
    stream::quic::{CacheManager, WebhookReceiver},
};
use anyhow::Error;
use common::ncryptflib as ncryptf;
use migration::{Migrator, MigratorTrait};
use rocket::http::Method;
use rocket::{self, catchers, routes};
use rocket_cors::{AllowedOrigins, CorsOptions};
use sea_orm_rocket::Database;
use std::sync::{Arc, Mutex};

/// Manager for the Rocket HTTP server
pub struct RocketManager {
    config: ApplicationConfig,
    webhook_receiver: WebhookReceiver,
    cache_manager: CacheManager,
    player_registrar: PlayerRegistrarService,
    identity_service: PlayerIdentityService,
    audio_playback_service: Arc<AudioPlaybackService>,
    bedrock_event_service: Arc<BedrockEventService>,
    cert_service: Arc<CertificateService>,
    hytale_session_cache: routes::api::HytaleSessionCache,
    audio_stream_token_cache: AudioStreamTokenCache,
    server_peer_store: Option<Arc<crate::relay::ServerPeerStore>>,
    relay_inject_delivery: Option<Arc<dyn crate::relay::LocalInjectDelivery>>,
    metrics: Arc<crate::services::MetricsService>,
    readiness: Arc<crate::runtime::ReadinessState>,
    shutdown_handle: Arc<Mutex<Option<rocket::Shutdown>>>,
    #[cfg(feature = "bedrock")]
    transfer_target_cache: crate::services::bedrock::TransferTargetCache,
}

impl RocketManager {
    pub fn new(
        config: ApplicationConfig,
        webhook_receiver: WebhookReceiver,
        cache_manager: CacheManager,
        player_registrar: PlayerRegistrarService,
        identity_service: PlayerIdentityService,
        audio_playback_service: Arc<AudioPlaybackService>,
        bedrock_event_service: Arc<BedrockEventService>,
        cert_service: Arc<CertificateService>,
        server_peer_store: Option<Arc<crate::relay::ServerPeerStore>>,
        relay_inject_delivery: Option<Arc<dyn crate::relay::LocalInjectDelivery>>,
        audio_stream_token_cache: Option<AudioStreamTokenCache>,
        metrics: Arc<crate::services::MetricsService>,
        readiness: Arc<crate::runtime::ReadinessState>,
        #[cfg(feature = "bedrock")]
        transfer_target_cache: crate::services::bedrock::TransferTargetCache,
    ) -> Self {
        Self {
            config,
            webhook_receiver,
            cache_manager,
            player_registrar,
            identity_service,
            audio_playback_service,
            bedrock_event_service,
            cert_service,
            hytale_session_cache: routes::api::HytaleSessionCache::new(),
            audio_stream_token_cache: audio_stream_token_cache
                .unwrap_or_else(AudioStreamTokenCache::new),
            server_peer_store,
            relay_inject_delivery,
            metrics,
            readiness,
            shutdown_handle: Arc::new(Mutex::new(None)),
            #[cfg(feature = "bedrock")]
            transfer_target_cache,
        }
    }


    /// Starts the Rocket HTTP server - this is the main entry point
    pub async fn start(&self) -> Result<(), Error> {
        tracing::info!("Starting Rocket HTTP server manager");

        // Ensure the assets directory exists
        let assets_path = std::path::Path::new(&self.config.server.assets_path);
        if !assets_path.exists() {
            tracing::info!(
                "Assets directory does not exist, creating: {:?}",
                assets_path
            );
            if let Err(e) = std::fs::create_dir_all(assets_path) {
                tracing::warn!("Failed to create assets directory: {}", e);
            }
        }

        match self.config.get_rocket_config() {
            Ok(figment) => {
                let cache = cached::TimedCache::with_lifespan_and_refresh(
                    std::time::Duration::from_secs(3600),
                    true,
                );
                let cache = Arc::new(Mutex::new(cache));
                let cache_wrapper = ncryptf::rocket::CacheWrapper::TimedCache(cache);

                let cors_config = &self.config.server.cors;
                let allowed_origins = if cors_config.allowed_origins.is_empty() {
                    AllowedOrigins::all()
                } else {
                    AllowedOrigins::some_exact(&cors_config.allowed_origins)
                };
                let cors = CorsOptions::default()
                    .allowed_origins(allowed_origins)
                    .allowed_methods(
                        vec![Method::Get, Method::Post, Method::Patch]
                            .into_iter()
                            .map(From::from)
                            .collect(),
                    )
                    .allow_credentials(cors_config.allow_credentials);

                let mut rocket = rocket::custom(figment)
                    .manage(crate::services::HealthService::new_shared(
                        self.readiness.clone(),
                        self.config.server.tls.certificate.clone(),
                    ))
                    .manage(cache_wrapper)
                    .manage(self.config.server.clone())
                    .manage(self.config.voice.clone())
                    .manage(self.config.server.features.clone())
                    .manage(self.webhook_receiver.clone())
                    .manage(self.cache_manager.clone())
                    .manage(self.player_registrar.clone())
                    .manage(self.identity_service.clone())
                    .manage(self.audio_playback_service.clone())
                    .manage(self.bedrock_event_service.clone())
                    .manage(self.cert_service.clone())
                    .manage(self.config.permissions.clone())
                    .manage(self.config.audio.clone())
                    .manage(self.hytale_session_cache.clone())
                    .manage(self.audio_stream_token_cache.clone())
                    .manage(self.metrics.clone());

                #[cfg(feature = "bedrock")]
                {
                    rocket = rocket.manage(self.transfer_target_cache.clone());
                }

                // Cross-server peering routes: the code-mint/offer, code-redeem, and
                // peer-link endpoints two servers sharing a realm use directly.
                // Discovery is decentralized (in-realm `!bvca` announce); there is no
                // central relay role to mount. Present whenever the relay plane built.
                if let (Some(store), Some(inject)) =
                    (&self.server_peer_store, &self.relay_inject_delivery)
                {
                    tracing::info!(
                        "relay plane active, mounting /relay/{{offer,peer-redeem,peer-link}}"
                    );
                    rocket = rocket.manage(store.clone()).manage(inject.clone()).mount(
                        "/api/relay",
                        routes![
                            routes::api::relay::offer::offer,
                            routes::api::relay::peer_redeem::peer_redeem,
                            routes::api::relay::peer_link::peer_link,
                        ],
                    );
                }

                let mut rocket = rocket
                    .attach(AppDb::init())
                    .attach(cors.to_cors().unwrap())
                    .attach(rocket::fairing::AdHoc::try_on_ignite("Migrations", migrate))
                    .mount(
                        "/assets",
                        rocket::fs::FileServer::from(&self.config.server.assets_path),
                    )
                    .mount(
                        "/assets",
                        routes![routes::assets::get_avatar, routes::assets::get_canvas,],
                    )
                    .mount("/ncryptf", routes![routes::ncryptf::ncryptf_ek_route])
                    .mount("/metrics", routes![routes::metrics::metrics]);

                for (prefix, route_list) in crate::http::openapi::OpenApiSpec::routes() {
                    rocket = rocket.mount(prefix, route_list);
                }

                if self.config.server.features.openapi_docs {
                    let spec = crate::http::openapi::OpenApiSpec::generate();
                    let spec_route = rocket_okapi::handlers::OpenApiHandler::new(spec)
                        .into_route("/openapi.json");
                    rocket = rocket
                        .mount("/", vec![spec_route])
                        .mount("/docs", routes![routes::docs::scalar_ui]);
                    tracing::info!("OpenAPI docs enabled at /docs");
                }

                let rocket = rocket.register(
                    "/",
                    catchers![
                        routes::catchers::default_catcher,
                        rocket_governor::rocket_governor_catcher
                    ],
                );

                match rocket.ignite().await {
                    Ok(ignite) => {
                        *self.shutdown_handle.lock().unwrap() = Some(ignite.shutdown());
                        tracing::info!("Rocket server is now running and awaiting requests!");
                        let result = ignite.launch().await;
                        if let Err(e) = result {
                            return Err(anyhow::anyhow!("Rocket launch error: {}", e));
                        }
                        Ok(())
                    }
                    Err(e) => Err(anyhow::anyhow!("Rocket ignite error: {}", e)),
                }
            }
            Err(error) => Err(anyhow::anyhow!("Rocket config error: {}", error)),
        }
    }

    /// Gracefully stops the running Rocket instance, if any. `start()` then
    /// returns Ok, letting the runtime decide whether to relaunch
    /// (certificate renewal) or shut down. Takes `&self` so it can be called
    /// while a `start()` future is still being polled.
    pub async fn stop(&self) -> Result<(), Error> {
        tracing::info!("Stopping Rocket HTTP server");
        if let Some(handle) = self.shutdown_handle.lock().unwrap().take() {
            handle.notify();
        }
        Ok(())
    }
}

/// The runtime supervises the long-running `start()` future directly in its
/// select loop (structured cancellation), so the inherent methods take
/// `&self` — this impl exists so RocketManager satisfies the same lifecycle
/// contract as every other streaming component.
impl common::traits::StreamTrait for RocketManager {
    /// Running means a live Shutdown handle is stashed; `stop()` takes it.
    fn is_stopped(&self) -> bool {
        self.shutdown_handle.lock().unwrap().is_none()
    }

    async fn metadata(&mut self, _key: String, _value: String) -> Result<(), Error> {
        Ok(())
    }

    async fn start(&mut self) -> Result<(), Error> {
        RocketManager::start(self).await
    }

    async fn stop(&mut self) -> Result<(), Error> {
        RocketManager::stop(self).await
    }
}

/// Migrate the database
async fn migrate(rocket: rocket::Rocket<rocket::Build>) -> rocket::fairing::Result {
    let conn = match AppDb::fetch(&rocket) {
        Some(db) => &db.conn,
        None => {
            tracing::error!("Migration: Failed to fetch database connection from Rocket");
            return Err(rocket);
        }
    };

    match Migrator::up(conn, None).await {
        Ok(_) => tracing::info!("Migration: All migrations applied successfully"),
        Err(e) => tracing::error!("Migration: Failed to run migrations: {}", e),
    }
    Ok(rocket)
}
