pub struct BvcpCodec;

impl BvcpCodec {
    const BVCP_PREFIX: &str = "!bvcp ";
    const BVCA_PREFIX: &str = "!bvca ";

    pub fn format_bvcp(token: &str) -> String {
        format!("{}{}", Self::BVCP_PREFIX, token)
    }

    pub fn parse_bvcp(message: &str) -> Option<String> {
        let rest = message.strip_prefix(Self::BVCP_PREFIX)?;
        let token = rest.trim();
        if token.is_empty() || token.contains(char::is_whitespace) {
            return None;
        }
        Some(token.to_string())
    }

    pub fn format_bvca(endpoint: &str) -> String {
        format!("{}{}", Self::BVCA_PREFIX, endpoint)
    }

    pub fn parse_bvca(message: &str) -> Option<String> {
        let rest = message.strip_prefix(Self::BVCA_PREFIX)?;
        let endpoint = rest.trim();
        if endpoint.is_empty() || endpoint.contains(char::is_whitespace) {
            return None;
        }
        Some(endpoint.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_token() {
        assert_eq!(BvcpCodec::format_bvcp("tok"), "!bvcp tok");
    }

    #[test]
    fn parses_valid_token() {
        assert_eq!(BvcpCodec::parse_bvcp("!bvcp tok"), Some("tok".to_string()));
    }

    #[test]
    fn rejects_non_bvcp_message() {
        assert_eq!(BvcpCodec::parse_bvcp("hello"), None);
    }

    #[test]
    fn rejects_bvce_eject_command() {
        assert_eq!(BvcpCodec::parse_bvcp("!bvce 1 2 3"), None);
    }

    #[test]
    fn round_trips_format_then_parse() {
        let token = "tok-abc123";
        let formatted = BvcpCodec::format_bvcp(token);
        assert_eq!(BvcpCodec::parse_bvcp(&formatted), Some(token.to_string()));
    }

    #[test]
    fn round_trips_announce() {
        let ep = "relay.example.com:443";
        let formatted = BvcpCodec::format_bvca(ep);
        assert_eq!(formatted, "!bvca relay.example.com:443");
        assert_eq!(BvcpCodec::parse_bvca(&formatted), Some(ep.to_string()));
    }

    #[test]
    fn parse_bvca_rejects_bvcp() {
        assert_eq!(BvcpCodec::parse_bvca("!bvcp tok"), None);
    }

    #[test]
    fn parse_bvcp_rejects_bvca() {
        assert_eq!(BvcpCodec::parse_bvcp("!bvca host:1"), None);
    }
}
