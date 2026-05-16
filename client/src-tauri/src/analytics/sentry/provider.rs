use async_trait::async_trait;

use crate::analytics::AnalyticsLevel;
use crate::analytics::dtos::QueuedEvent;
use crate::analytics::provider::AnalyticsProvider;

pub struct Provider;

impl Provider {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl AnalyticsProvider for Provider {
    fn handles_batches(&self) -> bool {
        false
    }

    fn set_tag(&self, key: &str, value: &str) {
        sentry::configure_scope(|scope| {
            scope.set_tag(key, value);
        });
    }

    fn clear_tag(&self, key: &str) {
        sentry::configure_scope(|scope| {
            scope.remove_tag(key);
        });
    }

    fn set_user(&self, user_id: &str) {
        sentry::configure_scope(|scope| {
            scope.set_user(Some(sentry::User {
                id: Some(user_id.to_string()),
                ..Default::default()
            }));
        });
    }

    fn breadcrumb(&self, category: &str, message: &str, level: AnalyticsLevel) {
        sentry::add_breadcrumb(sentry::Breadcrumb {
            category: Some(category.to_string()),
            message: Some(message.to_string()),
            level: level.into(),
            ..Default::default()
        });
    }

    fn capture_message(&self, message: &str, level: AnalyticsLevel, tags: &[(String, String)]) {
        sentry::with_scope(
            |scope| {
                for (k, v) in tags {
                    scope.set_tag(k, v);
                }
            },
            || {
                sentry::capture_message(message, level.into());
            },
        );
    }

    async fn send_batch(
        &self,
        _events: &[QueuedEvent],
        _install_id: &str,
        _session_id: &str,
    ) -> Result<(), anyhow::Error> {
        Ok(())
    }
}
