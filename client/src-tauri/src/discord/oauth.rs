use common::errors::DiscordLinkError;

pub struct DiscordOAuth;

impl DiscordOAuth {
    pub const SCOPE: &'static str = "guilds.members.read";

    pub fn authorize_url(client_id: &str, redirect_uri: &str, state: &str) -> String {
        let mut url = url::Url::parse("https://discord.com/oauth2/authorize")
            .expect("static authorize URL is valid");
        url.query_pairs_mut()
            .append_pair("client_id", client_id)
            .append_pair("response_type", "token")
            .append_pair("scope", Self::SCOPE)
            .append_pair("redirect_uri", redirect_uri)
            .append_pair("state", state)
            .append_pair("prompt", "consent");
        url.to_string()
    }

    // CSRF state: 256 bits of CSPRNG entropy, URL-safe base64. Stronger than a
    // v4 UUID (122 bits) and the right shape for a security token.
    pub fn generate_state() -> String {
        use base64::Engine;
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(rand::random::<[u8; 32]>())
    }

    pub fn parse_fragment(fragment: &str) -> Result<(String, String), DiscordLinkError> {
        let mut token = None;
        let mut state = None;
        for pair in fragment.trim_start_matches('#').split('&') {
            let mut it = pair.splitn(2, '=');
            match (it.next(), it.next()) {
                (Some("access_token"), Some(v)) => token = Some(v.to_string()),
                (Some("state"), Some(v)) => state = Some(v.to_string()),
                _ => {}
            }
        }
        match (token, state) {
            (Some(t), Some(s)) => Ok((t, s)),
            (None, _) => Err(DiscordLinkError::TokenMissing),
            (_, None) => Err(DiscordLinkError::StateMismatch),
        }
    }
}

impl DiscordOAuth {
    // Opens the authorize URL in the default browser. The redirect lands on the
    // hosted trampoline page, which bounces to `bedrock-voice-chat://discord-callback`;
    // the deep-link plugin then routes it to DiscordLinkService::complete_link.
    // Identical on desktop and mobile.
    pub fn open_external(app: &tauri::AppHandle, authorize_url: &str) -> Result<(), DiscordLinkError> {
        use tauri_plugin_opener::OpenerExt;
        app.opener()
            .open_url(authorize_url.to_string(), None::<String>)
            .map_err(|e| DiscordLinkError::Http(e.to_string()))
    }
}
