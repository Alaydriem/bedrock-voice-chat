use serde::Deserialize;

fn default_image() -> String {
    "24.04".to_string()
}

fn default_image_protocol() -> String {
    "simplestreams".to_string()
}

fn default_image_server() -> String {
    "https://cloud-images.ubuntu.com/releases".to_string()
}

/// LXD-wide settings shared by every target: the client identity trusted by the
/// LXD daemons, the container image to launch, and an optional cloud-init file
/// installing the bot runtime dependencies.
#[derive(Debug, Clone, Deserialize)]
pub struct LxdConfig {
    /// Client cert PEM (path) trusted via `lxc config trust add` on each daemon.
    pub client_cert: String,
    /// Client key PEM (path) for the identity above.
    pub client_key: String,
    /// Image alias to launch, e.g. `24.04`.
    #[serde(default = "default_image")]
    pub image: String,
    /// Image server protocol (`simplestreams` for the Ubuntu image server).
    #[serde(default = "default_image_protocol")]
    pub image_protocol: String,
    /// Image server URL.
    #[serde(default = "default_image_server")]
    pub image_server: String,
    /// Optional cloud-init user-data file applied at container launch.
    pub cloud_init: Option<String>,
}
