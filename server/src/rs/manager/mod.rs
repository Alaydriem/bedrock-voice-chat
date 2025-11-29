use crate::{
    config::ApplicationConfig,
    rs::pool::AppDb,
    rs::routes,
    stream::quic::{CacheManager, WebhookReceiver},
};
use anyhow::Error;
use common::ncryptflib as ncryptf;
use migration::{Migrator, MigratorTrait};
use moka::future::Cache;
use rocket::http::Method;
use rocket::{self, routes};
use rocket_cors::{AllowedOrigins, CorsOptions};
use sea_orm_rocket::Database;
use std::sync::{Arc, Mutex};

// Generate the ncryptf encryption key route at module level
ncryptf::ek_route!();

/// Manager for the Rocket HTTP server
pub struct RocketManager {
    config: ApplicationConfig,
    webhook_receiver: WebhookReceiver,
    channel_cache: Arc<async_mutex::Mutex<Cache<String, common::structs::channel::Channel>>>,
    cache_manager: CacheManager,
}

impl RocketManager {
    /// Creates a new RocketManager with the given configuration and dependencies
    pub fn new(
        config: ApplicationConfig,
        webhook_receiver: WebhookReceiver,
        channel_cache: Arc<async_mutex::Mutex<Cache<String, common::structs::channel::Channel>>>,
        cache_manager: CacheManager,
    ) -> Self {
        Self {
            config,
            webhook_receiver,
            channel_cache,
            cache_manager,
        }
    }

    /// Starts the Rocket HTTP server - this is the main entry point
    pub async fn start(&self) -> Result<(), Error> {
        tracing::info!("Starting Rocket HTTP server manager");

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

                let rocket = rocket::custom(figment)
                    .manage(cache_wrapper)
                    .manage(self.config.server.clone())
                    .manage(self.webhook_receiver.clone())
                    .manage(self.channel_cache.clone())
                    .manage(self.cache_manager.clone())
                    .attach(AppDb::init())
                    .attach(cors.to_cors().unwrap())
                    .attach(rocket::fairing::AdHoc::try_on_ignite("Migrations", migrate))
                    .mount("/assets", rocket::fs::FileServer::from("assets"))
                    .mount(
                        "/api",
                        routes![
                            routes::api::authenticate,
                            routes::api::get_config,
                            routes::api::update_position,
                            routes::api::position,
                            routes::api::pong
                        ],
                    )
                    .mount(
                        "/ncryptf",
                        routes![
                            ncryptf_ek_route //routes::ncryptf::token_info_route,
                                             //routes::ncryptf::token_revoke_route,
                                             //routes::ncryptf::token_refresh_route
                        ],
                    )
                    .mount(
                        "/api/channel",
                        routes![
                            routes::api::channel_create,
                            routes::api::channel_delete,
                            routes::api::channel_event,
                            routes::api::channel_list
                        ],
                    );

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
            return Err(rocket);
        }
    };

    let _ = Migrator::up(conn, None).await;
    Ok(rocket)
}
