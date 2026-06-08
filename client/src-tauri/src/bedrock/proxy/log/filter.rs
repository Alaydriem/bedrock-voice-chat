use std::sync::OnceLock;

const TARGET_PREFIXES: &[&str] = &[
    "bedrock_protocol",
    "bedrock_server",
    "bedrock_network",
    "rust_raknet",
    "raknet",
    "rakrs",
    "bvc_client_lib::bedrock",
    "bedrock_voice_chat_client::bedrock",
];

// Shared target/level predicates for the Bedrock log + tracing capture paths.
pub struct LogFilter;

impl LogFilter {
    // Verbosity floor for the Bedrock capture paths, read once from the
    // LOG_LEVEL env var (same knob tauri_plugin_log honors) so that
    // LOG_LEVEL=debug surfaces bedrock_protocol debug/trace events instead of
    // being silently dropped here. Defaults to INFO when unset/unparseable.
    fn tracing_threshold() -> tracing::Level {
        static LEVEL: OnceLock<tracing::Level> = OnceLock::new();
        *LEVEL.get_or_init(|| {
            std::env::var("LOG_LEVEL")
                .ok()
                .and_then(|s| s.parse::<tracing::Level>().ok())
                .unwrap_or(tracing::Level::INFO)
        })
    }

    fn log_threshold() -> log::LevelFilter {
        static LEVEL: OnceLock<log::LevelFilter> = OnceLock::new();
        *LEVEL.get_or_init(|| {
            std::env::var("LOG_LEVEL")
                .ok()
                .and_then(|s| s.parse::<log::LevelFilter>().ok())
                .unwrap_or(log::LevelFilter::Info)
        })
    }

    pub fn target_matches(target: &str) -> bool {
        TARGET_PREFIXES.iter().any(|p| target.starts_with(p))
    }

    pub fn log_level_allowed(level: log::Level) -> bool {
        level <= Self::log_threshold()
    }

    // tracing::Level Ord is inverted: ERROR is the lowest, TRACE the highest,
    // so "at least as severe as the threshold" is `event_level <= threshold`.
    pub fn tracing_level_allowed(level: &tracing::Level) -> bool {
        *level <= Self::tracing_threshold()
    }
}
