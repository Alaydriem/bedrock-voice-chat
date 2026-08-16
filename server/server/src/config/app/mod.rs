mod audio;
mod database;
mod logger;
mod permissions;
pub mod server;
mod voice;

pub use audio::Audio;
pub use database::Database;
pub use logger::Logger;
pub use permissions::Permissions;
pub use server::Acme;
pub use server::AcmeProviderKind;
pub use server::BedrockConfig;
pub use server::BedrockServerEntry;
pub use server::Features;
pub use server::Meridian;
pub use server::Minecraft;
pub use server::PeerConfig;
pub use server::Server;
pub use server::Tls;
pub use voice::Voice;

use common::ncryptflib::randombytes_buf;
use rocket::{
    data::{Limits, ToByteUnit},
    figment::Figment,
};

use anyhow::anyhow;
use sea_orm::{ConnectOptions, DatabaseConnection};
use std::net::SocketAddr;
use serde::{Deserialize, Serialize};
use tracing::Level;

/// Application Configuration as described in homemaker.hcl configuration file
#[derive(Serialize, Deserialize, Debug, Clone, schemars::JsonSchema)]
pub struct ApplicationConfig {
    #[serde(default)]
    pub database: Database,
    pub server: Server,
    #[serde(default)]
    pub log: Logger,
    #[serde(default)]
    pub voice: Voice,
    #[serde(default)]
    pub audio: Audio,
    #[serde(default)]
    pub permissions: Permissions,
}

impl Default for ApplicationConfig {
    fn default() -> Self {
        ApplicationConfig {
            database: Database::default(),
            server: Server::default(),
            voice: Voice::default(),
            log: Logger::default(),
            audio: Audio::default(),
            permissions: Permissions::default(),
        }
    }
}

impl ApplicationConfig {
    /// Parses an HCL document into an ApplicationConfig, evaluating
    /// `${env.VAR}` expressions against the provided variable map. A
    /// referenced-but-unset variable is a hard error — never a silent
    /// empty string.
    pub fn from_hcl_str_with_env(
        content: &str,
        env: &std::collections::HashMap<String, String>,
    ) -> Result<Self, anyhow::Error> {
        let mut ctx = hcl::eval::Context::new();
        let env_object: hcl::Map<String, hcl::Value> = env
            .iter()
            .map(|(k, v)| (k.clone(), hcl::Value::String(v.clone())))
            .collect();
        ctx.declare_var("env", hcl::Value::Object(env_object));

        let value: serde_json::Value = hcl::eval::from_str(content, &ctx)
            .map_err(|e| anyhow!("parsing configuration: {e}"))?;
        serde_json::from_value(value).map_err(|e| anyhow!("invalid configuration: {e}"))
    }

    /// Parses an HCL document, exposing the full process environment as the
    /// `env` object so any `${env.VAR}` reference resolves.
    pub fn from_hcl_str(content: &str) -> Result<Self, anyhow::Error> {
        Self::from_hcl_str_with_env(content, &std::env::vars().collect())
    }

    /// Parses the JSON an embedder supplies and applies environment overrides
    /// with the same precedence the CLI uses: env > config > serde default.
    /// The variable map is a parameter so callers can be tested without
    /// mutating process-global environment state.
    pub fn from_json_with_env(
        json: &str,
        vars: std::collections::HashMap<String, String>,
    ) -> Result<Self, anyhow::Error> {
        let config: Self =
            serde_json::from_str(json).map_err(|e| anyhow!("invalid configuration: {e}"))?;
        crate::config::EnvOverrides::from_vars(vars).apply(config)
    }

    /// Returns the database DSN string from the configuration.
    pub fn get_dsn(&self) -> String {
        self.database.get_dsn()
    }

    /// Returns the appropriate log level for Rocket.rs
    pub fn get_rocket_log_level(&self) -> rocket::config::LogLevel {
        match self.log.level.as_str() {
            "trace" => rocket::config::LogLevel::Debug,
            "debug" => rocket::config::LogLevel::Normal,
            "info" => rocket::config::LogLevel::Critical,
            "error" => rocket::config::LogLevel::Critical,
            "warn" => rocket::config::LogLevel::Critical,
            _ => rocket::config::LogLevel::Off,
        }
    }

    /// Returns the appropriate log level for tokio/tracing
    pub fn get_tracing_log_level(&self) -> tracing::Level {
        match self.log.level.as_str() {
            "info" => Level::INFO,
            "trace" => Level::TRACE,
            "debug" => Level::DEBUG,
            "warn" => Level::WARN,
            _ => Level::ERROR,
        }
    }

    /// The Rocket configuration, bound where `bind` says rather than where `server.port`
    /// does.
    ///
    /// The public port belongs to the TLS demultiplexer, which relays to this listener on
    /// loopback. `server.port` therefore names what a client dials, never what Rocket
    /// binds, and the two are no longer the same thing.
    pub fn get_rocket_config(&self, bind: SocketAddr) -> Result<Figment, anyhow::Error> {
        let cert_path = std::path::Path::new(&self.server.tls.certificate);
        let key_path = std::path::Path::new(&self.server.tls.key);

        if !cert_path.exists() {
            return Err(anyhow!(
                "TLS certificate not found at path: {}",
                cert_path.display()
            ));
        }

        if !key_path.exists() {
            return Err(anyhow!(
                "TLS private key not found at path: {}",
                key_path.display()
            ));
        }

        self.database.validate()?;
        tracing::info!("Database: {}", self.database.get_redacted_dsn());
        let figment = rocket::Config::figment()
            .merge(("cli_colors", false))
            .merge(("profile", rocket::figment::Profile::new("release")))
            .merge(("ident", false))
            .merge(("log_level", self.get_rocket_log_level()))
            .merge(("port", bind.port()))
            .merge(("address", bind.ip().to_string()))
            .merge(("limits", Limits::new().limit("json", (10).megabytes())))
            .merge(("secret_key", randombytes_buf(32)))
            .merge(("tls.certs", &self.server.tls.certificate))
            .merge(("tls.key", &self.server.tls.key))
            .merge((
                "tls.mutual.ca_certs",
                format!("{}/ca.crt", &self.server.tls.certs_path),
            ))
            .merge(("tls.mutual.mandatory", false))
            .merge(("shutdown.ctrlc", false))
            .merge((
                "minecraft.access_token",
                &self.server.minecraft.access_token,
            ))
            .merge((
                "databases.app",
                sea_orm_rocket::Config {
                    url: self.get_dsn().to_string(),
                    min_connections: None,
                    max_connections: 1024,
                    connect_timeout: 3,
                    idle_timeout: Some(1),
                    sqlx_logging: false,
                },
            ));

        Ok(figment)
    }

    /// Create a standalone database connection for CLI commands.
    pub async fn create_database_connection(&self) -> Result<DatabaseConnection, anyhow::Error> {
        self.database.validate()?;
        let dsn = self.database.get_dsn();

        let mut options = ConnectOptions::new(dsn);
        options
            .max_connections(10)
            .min_connections(1)
            .connect_timeout(std::time::Duration::from_secs(3))
            .idle_timeout(std::time::Duration::from_secs(60))
            .sqlx_logging(false);

        let conn = sea_orm::Database::connect(options).await?;
        Ok(conn)
    }
}
