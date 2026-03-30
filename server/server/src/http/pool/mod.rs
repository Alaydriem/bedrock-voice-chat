use rocket::async_trait;
use rocket::http::Status;
use rocket::request::{FromRequest, Outcome, Request};
use rocket_okapi::r#gen::OpenApiGenerator;
use rocket_okapi::request::{OpenApiFromRequest, RequestHeaderInput};
use sea_orm::{self, ConnectOptions};
use sea_orm_rocket::{self, rocket::figment::Figment, Config, Connection};

use std::ops::{Deref, DerefMut};
use std::time::Duration;
use tracing::log::LevelFilter;

#[derive(Debug, Clone)]
pub struct SeaOrmPool {
    pub conn: sea_orm::DatabaseConnection,
}

#[async_trait]
impl sea_orm_rocket::Pool for SeaOrmPool {
    type Error = sea_orm::DbErr;

    type Connection = sea_orm::DatabaseConnection;

    async fn init(figment: &Figment) -> Result<Self, Self::Error> {
        let config = figment.extract::<Config>().unwrap();
        let mut options: ConnectOptions = config.url.into();
        options
            .max_connections(config.max_connections as u32)
            .min_connections(config.min_connections.unwrap_or_default())
            .connect_timeout(Duration::from_secs(config.connect_timeout))
            .sqlx_logging_level(LevelFilter::Debug);

        if let Some(idle_timeout) = config.idle_timeout {
            options.idle_timeout(Duration::from_secs(idle_timeout));
        }

        let conn = sea_orm::Database::connect(options).await?;
        Ok(SeaOrmPool { conn })
    }

    fn borrow(&self) -> &Self::Connection {
        &self.conn
    }
}

#[derive(Debug)]
pub struct AppDb(SeaOrmPool);

impl From<SeaOrmPool> for AppDb {
    fn from(pool: SeaOrmPool) -> Self {
        AppDb(pool)
    }
}

impl Deref for AppDb {
    type Target = SeaOrmPool;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for AppDb {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl sea_orm_rocket::Database for AppDb {
    const NAME: &'static str = "app";
    type Pool = SeaOrmPool;
}

/// Newtype wrapper around `Connection<'r, AppDb>` that implements `OpenApiFromRequest`.
pub struct Db<'r>(Connection<'r, AppDb>);

impl<'r> Db<'r> {
    pub fn into_inner(self) -> &'r sea_orm::DatabaseConnection {
        self.0.into_inner()
    }
}

#[async_trait]
impl<'r> FromRequest<'r> for Db<'r> {
    type Error = <Connection<'r, AppDb> as FromRequest<'r>>::Error;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        Connection::from_request(req).await.map(Db)
    }
}

impl<'a> OpenApiFromRequest<'a> for Db<'a> {
    fn from_request_input(
        _gen: &mut OpenApiGenerator,
        _name: String,
        _required: bool,
    ) -> rocket_okapi::Result<RequestHeaderInput> {
        Ok(RequestHeaderInput::None)
    }
}
