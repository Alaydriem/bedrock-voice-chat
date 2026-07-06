use serde::Serialize;

use crate::structs::analytics::posthog::CaptureEvent;

#[derive(Debug, Clone, Serialize)]
pub struct BatchRequest<P: Serialize> {
    pub api_key: String,
    pub batch: Vec<CaptureEvent<P>>,
}
