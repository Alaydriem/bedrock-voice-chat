use serde::{Deserialize, Serialize};

fn default_certs_path() -> String {
    "./certificates".to_string()
}

fn default_tls_names() -> Vec<String> {
    vec!["localhost".to_string()]
}

fn default_tls_ips() -> Vec<String> {
    vec!["127.0.0.1".to_string()]
}

/// TLS Configuration for server
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApplicationConfigServerTLS {
    /// Path to TLS certificate (optional - if not set, uses auto-generated CA cert)
    #[serde(default)]
    pub certificate: String,
    /// Path to TLS private key (optional - if not set, uses auto-generated CA key)
    #[serde(default)]
    pub key: String,
    #[serde(default)]
    pub so_reuse_port: bool,
    #[serde(default = "default_certs_path")]
    pub certs_path: String,
    #[serde(default = "default_tls_names")]
    pub names: Vec<String>,
    #[serde(default = "default_tls_ips")]
    pub ips: Vec<String>,
}

impl Default for ApplicationConfigServerTLS {
    fn default() -> Self {
        Self {
            certificate: String::new(),
            key: String::new(),
            so_reuse_port: false,
            certs_path: default_certs_path(),
            names: default_tls_names(),
            ips: default_tls_ips(),
        }
    }
}
