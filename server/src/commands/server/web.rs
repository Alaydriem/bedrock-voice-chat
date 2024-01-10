use crate::{ rs::routes, config::ApplicationConfig };
use common::{ ncryptflib as ncryptf, pool::{ redis::RedisDb, seaorm::AppDb } };
use rocket::{ self, routes };
use rocket_db_pools;
use sea_orm_rocket::Database;
use tokio::task::JoinHandle;
use std::process::exit;
use migration::{ Migrator, MigratorTrait };

pub(crate) fn get_task(config: &ApplicationConfig) -> JoinHandle<()> {
    let app_config = config.to_owned();
    return tokio::task::spawn(async move {
        ncryptf::ek_route!(RedisDb);
        match app_config.get_rocket_config() {
            Ok(figment) => {
                let rocket = rocket
                    ::custom(figment)
                    .manage(app_config.server.clone())
                    .attach(AppDb::init())
                    .attach(RedisDb::init())
                    .attach(rocket::fairing::AdHoc::try_on_ignite("Migrations", migrate))
                    .mount(
                        "/api",
                        routes![
                            routes::api::authenticate,
                            routes::api::get_config,
                            routes::api::position,
                            routes::api::pong
                        ]
                    )
                    .mount(
                        "/ncryptf",
                        routes![
                            ncryptf_ek_route
                            //routes::ncryptf::token_info_route,
                            //routes::ncryptf::token_revoke_route,
                            //routes::ncryptf::token_refresh_route
                        ]
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
