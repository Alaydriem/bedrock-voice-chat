use serde::Deserialize;

use super::{AcmeSection, PlaySection};

#[derive(Debug, Default, Deserialize)]
pub struct TlsSection {
    #[serde(default)]
    pub names: Vec<String>,
    #[serde(default)]
    pub acme: Option<AcmeSection>,
    #[serde(default)]
    pub certificate: Option<String>,
    #[serde(default)]
    pub play: Option<PlaySection>,
}
