use crate::analytics::aptabase::Provider as AptabaseProvider;
use crate::analytics::dtos::QueuedEvent;

pub enum AnalyticsProviderType {
    Aptabase(AptabaseProvider),
}

impl AnalyticsProviderType {
    pub async fn send_batch(
        &self,
        events: &[QueuedEvent],
        install_id: &str,
        session_id: &str,
    ) -> Result<(), anyhow::Error> {
        match self {
            Self::Aptabase(p) => p.send_batch(events, install_id, session_id).await,
        }
    }
}
