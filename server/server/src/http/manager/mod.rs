use crate::{
    config::{ApplicationConfig, Permissions},
    http::pool::AppDb,
    http::routes,
    services::{AudioPlaybackService, CertificateService, PlayerIdentityService, PlayerRegistrarService},
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

// Generate the ncryptf encryption key route at module level
ncryptf::ek_route!();

/// Manager for the Rocket HTTP server
pub struct RocketManager {
    config: ApplicationConfig,
    webhook_receiver: WebhookReceiver,
    cache_manager: CacheManager,
    player_registrar: PlayerRegistrarService,
    identity_service: PlayerIdentityService,
    audio_playback_service: Arc<AudioPlaybackService>,
    cert_service: Arc<CertificateService>,
    hytale_session_cache: routes::api::HytaleSessionCache,
}

impl RocketManager {
    pub fn new(
        config: ApplicationConfig,
        webhook_receiver: WebhookReceiver,
        cache_manager: CacheManager,
        player_registrar: PlayerRegistrarService,
        identity_service: PlayerIdentityService,
        audio_playback_service: Arc<AudioPlaybackService>,
        cert_service: Arc<CertificateService>,
    ) -> Self {
        Self {
            config,
            webhook_receiver,
            cache_manager,
            player_registrar,
            identity_service,
            audio_playback_service,
            cert_service,
            hytale_session_cache: routes::api::HytaleSessionCache::new(),
        }
    }

    /// Starts the Rocket HTTP server - this is the main entry point
    pub async fn start(&self) -> Result<(), Error> {
        tracing::info!("Starting Rocket HTTP server manager");

        // Ensure the assets directory exists
        let assets_path = std::path::Path::new(&self.config.server.assets_path);
        if !assets_path.exists() {
            tracing::info!("Assets directory does not exist, creating: {:?}", assets_path);
            if let Err(e) = std::fs::create_dir_all(assets_path) {
                tracing::warn!("Failed to create assets directory: {}", e);
            }
        }

        match self.config.get_rocket_config() {
            Ok(figment) => {
                let cache = cached::TimedCache::with_lifespan_and_refresh(
                    std::time::Duration::from_secs(3600),
                    true
                );
                let cache = Arc::new(Mutex::new(cache));
                let cache_wrapper = ncryptf::rocket::CacheWrapper::TimedCache(cache);

                let cors = CorsOptions::default()
                    .allowed_origins(AllowedOrigins::all())
                    .allowed_methods(
                        vec![Method::Get, Method::Post, Method::Patch]
                            .into_iter()
                            .map(From::from)
                            .collect(),
                    )
                    .allow_credentials(true);

                let mut rocket = rocket::custom(figment)
                    .manage(cache_wrapper)
                    .manage(self.config.server.clone())
                    .manage(self.config.voice.clone())
                    .manage(self.config.server.features.clone())
                    .manage(self.webhook_receiver.clone())
                    .manage(self.cache_manager.clone())
                    .manage(self.player_registrar.clone())
                    .manage(self.identity_service.clone())
                    .manage(self.audio_playback_service.clone())
                    .manage(self.cert_service.clone())
                    .manage(self.config.permissions.clone())
                    .manage(self.config.audio.clone())
                    .manage(self.hytale_session_cache.clone())
                    .attach(AppDb::init())
                    .attach(cors.to_cors().unwrap())
                    .attach(rocket::fairing::AdHoc::try_on_ignite("Migrations", migrate))
                    .mount("/assets", rocket::fs::FileServer::from(&self.config.server.assets_path))
                    .mount("/assets", routes![
                        routes::assets::get_avatar,
                        routes::assets::get_canvas,
                    ])
                    .mount(
                        "/ncryptf",
                        routes![
                            ncryptf_ek_route
                        ],
                    );

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

                let rocket = rocket
                    .register("/", catchers![routes::catchers::default_catcher]);

                match rocket.ignite().await {
                    Ok(ignite) => {
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

    /// Stops the Rocket HTTP server gracefully
    pub async fn stop(&mut self) -> Result<(), Error> {
        tracing::info!("Stopping Rocket HTTP server");
        // Rocket doesn't have a direct stop method, but we can handle graceful shutdown
        // by returning from start() method
        Ok(())
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
