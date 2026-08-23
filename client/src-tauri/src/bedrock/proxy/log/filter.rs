const TARGET_PREFIXES: &[&str] = &[
    "bedrock_protocol",
    "bedrock_client",
    "bedrock_server",
    "bedrock_network",
    "rust_raknet",
    "raknet",
    "rakrs",
    "bvc_client_lib::bedrock",
    "bedrock_voice_chat_client::bedrock",
];

// Target predicate for the Bedrock capture path. The level half moved to the
// sink's registered level, which the Dispatcher applies before emit is called;
// LOG_LEVEL now sets that level where BedrockSink is constructed.
pub struct LogFilter;

impl LogFilter {
    pub fn target_matches(target: &str) -> bool {
        TARGET_PREFIXES.iter().any(|p| target.starts_with(p))
    }
}
