use crate::analytics::dtos::QueuedEvent;
use crate::analytics::posthog::Provider as PostHogProvider;

pub enum AnalyticsProviderType {
    PostHog(PostHogProvider),
}

impl AnalyticsProviderType {
    pub async fn send_batch(
        &self,
        events: &[QueuedEvent],
        install_id: &str,
        session_id: &str,
    ) -> Result<(), anyhow::Error> {
        match self {
            Self::PostHog(p) => p.send_batch(events, install_id, session_id).await,
        }
    }
}
