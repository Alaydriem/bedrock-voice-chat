use sea_orm::{self, ConnectOptions};
use sea_orm_rocket::{self, rocket::figment::Figment, Config};

use async_trait::async_trait;
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
