#[cfg(any(target_os = "windows", target_os = "macos"))]
mod keychain;
#[cfg(any(target_os = "windows", target_os = "macos"))]
pub use keychain::KeychainBackend as DefaultBackend;

#[cfg(target_os = "linux")]
mod file;
#[cfg(target_os = "linux")]
pub use file::FileBackend as DefaultBackend;

pub trait SecretBackend {
    fn save(&self, slot_key: &str, cert: &str, key: &str, ca: &str) -> Result<(), anyhow::Error>;
    fn load(&self, slot_key: &str) -> Result<(String, String, String), anyhow::Error>;
    fn delete(&self, slot_key: &str) -> Result<(), anyhow::Error>;
}

pub fn default_backend() -> DefaultBackend {
    DefaultBackend::new()
}
