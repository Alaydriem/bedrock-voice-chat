use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

/// THROWAWAY. The second candidate transport for the meter pivot, measured beside the first.
///
/// A custom URI scheme handler is intercepted inside the webview and answered by this process. It
/// never reaches the platform network stack, so neither Android's cleartext policy nor iOS App
/// Transport Security has an opinion about it — which is the whole reason it is worth measuring
/// against a loopback socket that both of those refuse by default.
///
/// The handler holds its responder rather than answering at once. That is what makes it a push
/// channel instead of a poll: a caller asks for the next snapshot and the answer arrives when
/// there is one, or when the hold expires.
pub struct PushProtocol;

/// Process-global, because the instance the builder closure owns and the instance managed for the
/// command to read are not the same object. A throwaway does not earn the wiring to make them one.
static SERVED: AtomicU32 = AtomicU32::new(0);

impl PushProtocol {
    /// The scheme name. Registered on the builder, so the webview reaches it at
    /// `http://bvcpush.localhost/...` on Windows and Android and `bvcpush://localhost/...` on
    /// macOS and iOS — a difference the page has to discover rather than assume.
    pub const SCHEME: &'static str = "bvcpush";

    /// How long a request is held before it is answered empty. Stands in for "the next snapshot",
    /// which the real thing would wait on.
    const HOLD: std::time::Duration = std::time::Duration::from_millis(300);

    pub fn new_shared() -> Arc<Self> {
        Arc::new(Self)
    }

    pub fn served(&self) -> u32 {
        SERVED.load(Ordering::Relaxed)
    }

    /// Answer one intercepted request, after a hold.
    ///
    /// The permissive origin header is not incidental. A page on one custom scheme calling
    /// another is cross-origin on every platform, and Tauri's own documentation names this as the
    /// thing that has to be handled for `fetch` to work at all.
    pub fn handle(self: &Arc<Self>, responder: tauri::UriSchemeResponder) {
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(Self::HOLD).await;
            let seq = SERVED.fetch_add(1, Ordering::Relaxed);
            let body = format!("{{\"type\":\"probe\",\"seq\":{}}}", seq);
            let response = tauri::http::Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .header("Access-Control-Allow-Origin", "*")
                .header("Cache-Control", "no-store")
                .body(body.into_bytes());

            match response {
                Ok(response) => responder.respond(response),
                Err(e) => log::warn!("push protocol: could not build a response: {}", e),
            }
        });
    }
}
