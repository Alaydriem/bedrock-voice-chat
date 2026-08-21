use serde_json::{Map, Value};

#[derive(Debug, Clone, Default)]
pub struct RoutedFields {
    pub tags: Vec<(String, String)>,
    pub attributes: Map<String, Value>,
    pub context: Map<String, Value>,
}
