use crate::analytics::AnalyticsLevel;
use crate::analytics::dtos::QueuedEvent;
use crate::analytics::posthog::Provider as PostHogProvider;
use crate::analytics::sentry::Provider as SentryProvider;

pub enum AnalyticsProviderType {
    PostHog(PostHogProvider),
    Sentry(SentryProvider),
}

impl AnalyticsProviderType {
    pub fn handles_batches(&self) -> bool {
        match self {
            Self::PostHog(_) => true,
            Self::Sentry(_) => false,
        }
    }

    pub async fn send_batch(
        &self,
        events: &[QueuedEvent],
        install_id: &str,
        session_id: &str,
    ) -> Result<(), anyhow::Error> {
        match self {
            Self::PostHog(p) => p.send_batch(events, install_id, session_id).await,
            Self::Sentry(p) => p.send_batch(events, install_id, session_id).await,
        }
    }

    pub fn set_tag(&self, key: &str, value: &str) {
        match self {
            Self::PostHog(p) => p.set_tag(key, value),
            Self::Sentry(p) => p.set_tag(key, value),
        }
    }

    pub fn clear_tag(&self, key: &str) {
        match self {
            Self::PostHog(p) => p.clear_tag(key),
            Self::Sentry(p) => p.clear_tag(key),
        }
    }

    pub fn set_user(&self, user_id: &str) {
        match self {
            Self::PostHog(p) => p.set_user(user_id),
            Self::Sentry(p) => p.set_user(user_id),
        }
    }

    pub fn breadcrumb(&self, category: &str, message: &str, level: AnalyticsLevel) {
        match self {
            Self::PostHog(p) => p.breadcrumb(category, message, level),
            Self::Sentry(p) => p.breadcrumb(category, message, level),
        }
    }

    pub fn capture_message(
        &self,
        message: &str,
        level: AnalyticsLevel,
        tags: &[(String, String)],
    ) {
        match self {
            Self::PostHog(p) => p.capture_message(message, level, tags),
            Self::Sentry(p) => p.capture_message(message, level, tags),
        }
    }
}
