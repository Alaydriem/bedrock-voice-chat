use serde::Serialize;

use crate::analytics::posthog::CaptureEvent;

#[derive(Debug, Clone, Serialize)]
pub struct BatchRequest {
    pub api_key: String,
    pub batch: Vec<CaptureEvent>,
}
