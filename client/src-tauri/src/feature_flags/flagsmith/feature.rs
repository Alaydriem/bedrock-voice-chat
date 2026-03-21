use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct FlagsmithFeature {
    pub name: String,
}
