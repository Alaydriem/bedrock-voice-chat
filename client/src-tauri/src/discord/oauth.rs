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

#[cfg(desktop)]
impl DiscordOAuth {
    // Embedded webview that intercepts the navigation to `redirect_uri` and
    // returns the raw URL fragment (containing the implicit-grant access_token).
    // Mirrors crate::auth::mc_oauth_window::McOauthWindow.
    pub async fn open_window(
        app: tauri::AppHandle,
        authorize_url: String,
        redirect_uri: String,
    ) -> Result<String, DiscordLinkError> {
        use tauri::{Manager, Url, webview::WebviewWindowBuilder};
        use tokio::sync::oneshot;

        let (tx, rx) = oneshot::channel::<String>();
        let tx = std::sync::Mutex::new(Some(tx));

        let url: Url = authorize_url
            .parse()
            .map_err(|e| DiscordLinkError::Http(format!("invalid authorize URL: {e}")))?;

        let label = format!("discord-oauth-{}", uuid::Uuid::new_v4().as_simple());
        for (_, w) in app.webview_windows() {
            if w.label().starts_with("discord-oauth-") {
                let _ = w.destroy();
            }
        }

        let close_handle = app.clone();
        let close_label = label.clone();
        let redirect = redirect_uri.clone();

        let builder = WebviewWindowBuilder::new(&app, &label, tauri::WebviewUrl::External(url))
            .on_navigation(move |url: &Url| {
                if url.as_str().starts_with(&redirect) {
                    let fragment = url.fragment().unwrap_or("").to_string();
                    if let Some(sender) = tx.lock().unwrap().take() {
                        let _ = sender.send(fragment);
                    }
                    if let Some(w) = close_handle.get_webview_window(&close_label) {
                        let _ = w.destroy();
                    }
                    return false;
                }
                true
            })
            .title("Link Discord")
            .inner_size(500.0, 750.0)
            .center()
            .resizable(true);

        builder
            .build()
            .map_err(|e| DiscordLinkError::Http(format!("failed to open OAuth window: {e}")))?;

        rx.await.map_err(|_| DiscordLinkError::WindowClosed)
    }
}
