use crate::{config::ApplicationConfig, rs::routes};
use common::structs::packet::QuicNetworkPacket;
use common::{
    ncryptflib as ncryptf,
    pool::{redis::RedisDb, seaorm::AppDb},
};
use migration::{Migrator, MigratorTrait};
use moka::future::Cache;
use rocket::{self, routes};
use rocket_db_pools;
use rocket::http::Method;
use rocket_cors::{AllowedOrigins, CorsOptions};
use sea_orm_rocket::Database;
use std::process::exit;
use std::sync::Arc;
use tokio::task::JoinHandle;

pub(crate) fn get_task(
    config: &ApplicationConfig,
    queue: Arc<deadqueue::limited::Queue<QuicNetworkPacket>>,
    channel_cache: Arc<async_mutex::Mutex<Cache<String, common::structs::channel::Channel>>>,
) -> JoinHandle<()> {
    let app_config = config.to_owned();
    return tokio::task::spawn(async move {
        ncryptf::ek_route!(RedisDb);
        match app_config.get_rocket_config() {
            Ok(figment) => {
                let cors = CorsOptions::default()
                .allowed_origins(AllowedOrigins::all())
                .allowed_methods(
                    vec![Method::Get, Method::Post, Method::Patch]
                        .into_iter()
                        .map(From::from)
                        .collect(),
                ).allow_credentials(true);

                let rocket = rocket::custom(figment)
                    .manage(app_config.server.clone())
                    .manage(queue.clone())
                    .manage(channel_cache.clone())
                    .attach(AppDb::init())
                    .attach(RedisDb::init())
                    .attach(cors.to_cors().unwrap())
                    .attach(rocket::fairing::AdHoc::try_on_ignite("Migrations", migrate))
                    .mount("/assets", rocket::fs::FileServer::from("assets"))
                    .mount(
                        "/api",
                        routes![
                            routes::api::authenticate,
                            routes::api::get_config,
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

                if let Ok(ignite) = rocket.ignite().await {
                    info!("Rocket server is now running and awaiting request!");
                    let result = ignite.launch().await;
                    if result.is_err() {
                        println!("{}", result.unwrap_err());
                        exit(1);
                    }
                }
            }
            Err(error) => {
                println!("{}", error);
                exit(1);
            }
        }
    });
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
