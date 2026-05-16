use async_trait::async_trait;
use enum_dispatch::enum_dispatch;

use crate::analytics::AnalyticsLevel;
use crate::analytics::dtos::QueuedEvent;
use crate::analytics::posthog::Provider as PostHogProvider;
use crate::analytics::sentry::Provider as SentryProvider;

#[async_trait]
#[enum_dispatch]
pub trait AnalyticsProvider {
    fn handles_batches(&self) -> bool {
        true
    }

    async fn send_batch(
        &self,
        events: &[QueuedEvent],
        install_id: &str,
        session_id: &str,
    ) -> Result<(), anyhow::Error>;

    fn set_tag(&self, key: &str, value: &str);

    fn clear_tag(&self, key: &str);

    fn set_user(&self, user_id: &str);

    fn breadcrumb(&self, category: &str, message: &str, level: AnalyticsLevel);

    fn capture_message(&self, message: &str, level: AnalyticsLevel, tags: &[(String, String)]);
}

#[enum_dispatch(AnalyticsProvider)]
pub enum AnalyticsProviderType {
    PostHog(PostHogProvider),
    Sentry(SentryProvider),
}
