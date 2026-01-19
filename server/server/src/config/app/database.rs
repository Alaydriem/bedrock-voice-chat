use serde::{Deserialize, Serialize};

fn default_db_scheme() -> String {
    "sqlite3".to_string()
}

fn default_db_database() -> String {
    "./bvc.sqlite3".to_string()
}

/// Database configuration for MySQL/MariaDB/SQLite
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApplicationConfigDatabase {
    #[serde(default = "default_db_scheme")]
    pub scheme: String,
    #[serde(default = "default_db_database")]
    pub database: String,
    pub host: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    #[serde(default)]
    pub port: Option<u32>,
}

impl Default for ApplicationConfigDatabase {
    fn default() -> Self {
        Self {
            scheme: default_db_scheme(),
            database: default_db_database(),
            host: None,
            username: None,
            password: None,
            port: None,
        }
    }
}
