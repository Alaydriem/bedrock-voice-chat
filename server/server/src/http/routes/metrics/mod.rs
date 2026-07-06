use std::sync::Arc;

use rocket::State;
use rocket::http::ContentType;

use crate::services::MetricsService;

#[get("/")]
pub async fn metrics(metrics: &State<Arc<MetricsService>>) -> (ContentType, String) {
    (ContentType::Plain, metrics.render())
}
