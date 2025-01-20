use common::ncryptflib::randombytes_buf;
use rocket::{ data::{ Limits, ToByteUnit }, figment::Figment };

use anyhow::anyhow;
use serde::{ Deserialize, Serialize };
use tracing::Level;

/// Application Configuration as described in homemaker.hcl configuration file
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApplicationConfig {
    pub database: ApplicationConfigDatabase,
    pub redis: ApplicationConfigRedis,
    pub server: ApplicationConfigServer,
    pub log: ApplicationConfigLogger,
    pub voice: ApplicationConfigVoice,
}

/// Database configuration for MySQL/MariaDB
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApplicationConfigDatabase {
    pub scheme: String,
    pub database: String,
    pub host: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    #[serde(default)]
    pub port: Option<u32>,
}

/// Database configuration for Redis
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApplicationConfigRedis {
    #[serde(default)]
    pub database: String,
    pub host: String,
    #[serde(default)]
    pub port: u32,
}


/// Common server configuration
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApplicationConfigServer {
    #[serde(default)]
    pub listen: String,
    #[serde(default)]
    pub port: u32,
    #[serde(default)]
    pub quic_port: u32,
    #[serde(default)]
    pub public_addr: String,
    pub tls: ApplicationConfigServerTLS,
    pub minecraft: ApplicationConfigMinecraft,
}

/// Voice specific settings
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApplicationConfigVoice {
    #[serde(default)]
    pub broadcast_range: f32,
    #[serde(default)]
    pub crouch_distance_multiplier: f32,
    #[serde(default)]
    pub whisper_distance_multiplier: f32,
}

/// TLS Configuration for server
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApplicationConfigServerTLS {
    certificate: String,
    key: String,
    #[serde(default)]
    pub so_reuse_port: bool,
    pub certs_path: String,
    pub names: Vec<String>,
    pub ips: Vec<String>,
}

/// Minecraft Configuration
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApplicationConfigMinecraft {
    pub access_token: String,
    pub client_id: String,
    pub client_secret: String,
}

/// OAuth2 Client Configuration
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApplicationConfigLogger {
    pub level: String,
    pub out: String,
}

/// Default values for ApplicationConfig struct
impl Default for ApplicationConfig {
    fn default() -> Self {
        ApplicationConfig {
            redis: ApplicationConfigRedis {
                database: String::from(""),
                host: String::from("127.0.0.1"),
                port: 6379,
            },
            database: ApplicationConfigDatabase {
                scheme: String::from("sqlite3"),
                database: String::from("/etc/bvc/bvc.sqlite3"),
                host: None,
                port: None,
                username: None,
                password: None,
            },
            server: ApplicationConfigServer {
                listen: String::from("127.0.0.1"),
                port: 443,
                quic_port: 8443,
                public_addr: String::from("127.0.0.1"),
                tls: ApplicationConfigServerTLS {
                    certificate: String::from("/etc/bvc/server.crt"),
                    key: String::from("/etc/bvc/server.key"),
                    so_reuse_port: false,
                    certs_path: String::from("/etc/bvc/certificates"),
                    names: vec!["localhost".to_string()],
                    ips: vec!["127.0.0.1".to_string()],
                },
                minecraft: ApplicationConfigMinecraft {
                    access_token: String::from(""),
                    client_id: String::from(""),
                    client_secret: String::from(""),
                },
            },
            voice: ApplicationConfigVoice {
                broadcast_range: 32.0,
                crouch_distance_multiplier: 1.0,
                whisper_distance_multiplier: 0.5,
            },
            log: ApplicationConfigLogger {
                level: String::from("info"),
                out: String::from("stdout"),
            },
        }
    }
}

impl ApplicationConfig {
    fn get_dsn<'a>(&'a self) -> String {
        match self.database.scheme.as_str() {
            "sqlite" | "sqlite3" => {
                let path = std::path::Path::new(&self.database.database);
                if !path.exists() {
                    match std::fs::File::create(&self.database.database) {
                        Ok(_) => {}
                        Err(_e) => {
                            panic!(
                                "Verify that {} exists and is writable. You may need to create this file.",
                                &self.database.database
                            );
                        }
                    }
                }

                format!("sqlite://{}", &self.database.database)
            }
            "mysql" =>
                format!(
                    "mysql://{}:{}@{}:{}/{}",
                    &self.database.username.clone().unwrap_or(String::from("")),
                    &self.database.password.clone().unwrap_or(String::from("")),
                    &self.database.host.clone().unwrap_or(String::from("127.0.0.1")),
                    &self.database.port.unwrap_or(3306),
                    &self.database.database
                ),
            _ => format!("sqlite://{}", "/etc/bvc/bvc.sqlite3"),
        }
    }

    /// Returns the appropriate log level for Rocket.rs
    pub fn get_rocket_log_level<'a>(&'a self) -> rocket::config::LogLevel {
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
    pub fn get_tracing_log_level<'a>(&'a self) -> tracing::Level {
        match self.log.level.as_str() {
            "info" => Level::INFO,
            "trace" => Level::TRACE,
            "debug" => Level::DEBUG,
            "warn" => Level::WARN,
            _ => Level::ERROR,
        }
    }

    pub fn get_rocket_config<'a>(&'a self) -> Result<Figment, anyhow::Error> {
        if
            !std::path::Path::new(&self.server.tls.certificate).exists() ||
            !std::path::Path::new(&self.server.tls.key).exists()
        {
            return Err(anyhow!("TLS certificate or private key is not valid"));
        }

        tracing::info!("Database: {}", self.get_dsn().to_string());
        let figment = rocket::Config
            ::figment()
            .merge(("profile", rocket::figment::Profile::new("release")))
            .merge(("ident", false))
            .merge(("log_level", self.get_rocket_log_level()))
            .merge(("port", &self.server.port))
            .merge(("address", &self.server.listen))
            .merge(("limits", Limits::new().limit("json", (10).megabytes())))
            .merge(("secret_key", randombytes_buf(32)))
            .merge(("tls.certs", &self.server.tls.certificate))
            .merge(("tls.key", &self.server.tls.key))
            .merge(("tls.mutual.ca_certs", format!("{}/ca.crt", &self.server.tls.certs_path)))
            .merge(("tls.mutual.mandatory", false))
            .merge(("minecraft.access_token", &self.server.minecraft.access_token))
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
            ))
            .merge((
                "databases.cache",
                rocket_db_pools::Config {
                    url: format!(
                        "redis://{}:{}/{}",
                        self.redis.host,
                        self.redis.port,
                        self.redis.database
                    ),
                    min_connections: None,
                    max_connections: 1024,
                    connect_timeout: 3,
                    idle_timeout: None,
                    extensions: None,
                },
            ));

        return Ok(figment);
    }
}
