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
    pub fn target_matches(target: &str) -> bool {
        TARGET_PREFIXES.iter().any(|p| target.starts_with(p))
    }

    pub fn log_level_allowed(level: log::Level) -> bool {
        matches!(level, log::Level::Info | log::Level::Warn | log::Level::Error)
    }

    pub fn tracing_level_allowed(level: &tracing::Level) -> bool {
        *level == tracing::Level::INFO
            || *level == tracing::Level::WARN
            || *level == tracing::Level::ERROR
    }
}
