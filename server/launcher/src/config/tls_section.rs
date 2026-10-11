use serde::Deserialize;

use super::AcmeSection;

#[derive(Debug, Default, Deserialize)]
pub struct TlsSection {
    #[serde(default)]
    pub names: Vec<String>,
    #[serde(default)]
    pub acme: Option<AcmeSection>,
}
