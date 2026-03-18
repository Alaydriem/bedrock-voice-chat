use tauri::Manager;
use tauri::webview::WebviewWindowBuilder;
use tauri::Url;
use tokio::sync::oneshot;

const MC_CLIENT_ID: &str = "00000000402b5328";
const REDIRECT_URI: &str = "https://login.live.com/oauth20_desktop.srf";
const REDIRECT_URI_ENCODED: &str = "https%3A%2F%2Flogin.live.com%2Foauth20_desktop.srf";

pub struct McOauthWindow;

impl McOauthWindow {
    pub async fn open(app_handle: tauri::AppHandle) -> Result<String, String> {
        let (tx, rx) = oneshot::channel::<String>();
        let tx = std::sync::Mutex::new(Some(tx));

        let oauth_url = format!(
            "https://login.live.com/oauth20_authorize.srf?client_id={}&response_type=code&redirect_uri={}&scope=XboxLive.signin%20offline_access",
            MC_CLIENT_ID,
            REDIRECT_URI_ENCODED,
        );

        let url: Url = oauth_url.parse().map_err(|e| format!("Invalid URL: {}", e))?;

        // Use a unique label so re-opening works if previous window wasn't cleaned up
        let label = format!("mc-oauth-{}", uuid::Uuid::new_v4().as_simple());

        // Close any leftover OAuth windows from previous attempts
        for (_, webview_window) in app_handle.webview_windows() {
            if webview_window.label().starts_with("mc-oauth-") {
                #[cfg(desktop)]
                let _ = webview_window.destroy();
            }
        }

        let close_handle = app_handle.clone();
        let close_label = label.clone();

        let mut builder = WebviewWindowBuilder::new(
            &app_handle,
            &label,
            tauri::WebviewUrl::External(url),
        )
        .on_navigation(move |url: &Url| {
            let url_str = url.as_str();
            if url_str.starts_with(REDIRECT_URI) {
                if let Some(code) = url
                    .query_pairs()
                    .find(|(k, _)| k == "code")
                    .map(|(_, v)| v.to_string())
                {
                    if let Some(sender) = tx.lock().unwrap().take() {
                        let _ = sender.send(code);
                    }
                }

                // Close the window immediately from the callback
                if let Some(w) = close_handle.get_webview_window(&close_label) {
                    #[cfg(desktop)]
                    let _ = w.destroy();
                }

                return false;
            }
            true
        });

        #[cfg(desktop)]
        {
            builder = builder
                .title("Link Java Identity")
                .inner_size(500.0, 700.0)
                .center()
                .resizable(true);
        }

        let _window = builder
            .build()
            .map_err(|e| format!("Failed to open OAuth window: {}", e))?;

        rx.await.map_err(|_| "OAuth window closed without completing".to_string())
    }

    pub fn redirect_uri() -> &'static str {
        REDIRECT_URI
    }

    pub fn client_id() -> &'static str {
        MC_CLIENT_ID
    }
}
