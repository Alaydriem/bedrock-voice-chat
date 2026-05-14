#[derive(Debug, Clone, Copy)]
pub enum AnalyticsLevel {
    Info,
    Warning,
    Error,
}

impl From<AnalyticsLevel> for sentry::Level {
    fn from(value: AnalyticsLevel) -> Self {
        match value {
            AnalyticsLevel::Info => sentry::Level::Info,
            AnalyticsLevel::Warning => sentry::Level::Warning,
            AnalyticsLevel::Error => sentry::Level::Error,
        }
    }
}
