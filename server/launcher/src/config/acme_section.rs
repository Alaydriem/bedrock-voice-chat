use serde::Deserialize;

#[derive(Debug, Default, Deserialize)]
pub struct AcmeSection {
    #[serde(default)]
    pub domains: Option<Vec<String>>,
}
