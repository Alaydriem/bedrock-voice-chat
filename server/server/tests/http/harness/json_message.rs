#[derive(Debug, Clone, serde::Deserialize)]
pub struct JsonMessage<T> {
    pub status: u16,
    pub data: Option<T>,
    pub message: Option<String>,
}
