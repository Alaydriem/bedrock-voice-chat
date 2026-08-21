use tokio::net::TcpListener;

/// Chooses the port the operator-facing listener answers on.
///
/// The configured port is a preference rather than a requirement. Anything else holding it — a
/// second BVC instance, an unrelated program — left the listener unbound for the rest of the
/// session, and an operator API that is not there at all is a worse answer than one that is
/// there on a neighbouring port and says so.
pub struct ListenerBinder;

impl ListenerBinder {
    /// How far past the preferred port the search goes.
    ///
    /// Bounded because a listener that wandered is one an operator cannot find. A machine with
    /// this whole block taken gets an error naming the range instead.
    const SEARCH_SPAN: u16 = 16;

    pub async fn bind(host: &str, preferred: u16) -> Result<TcpListener, anyhow::Error> {
        Self::bind_within(host, preferred, Self::SEARCH_SPAN).await
    }

    /// The search with its span given, which is what makes the bound testable.
    pub async fn bind_within(
        host: &str,
        preferred: u16,
        span: u16,
    ) -> Result<TcpListener, anyhow::Error> {
        let mut taken: Option<std::io::Error> = None;

        for offset in 0..=span {
            let Some(port) = preferred.checked_add(offset) else {
                break;
            };

            match TcpListener::bind((host, port)).await {
                Ok(listener) => {
                    if offset > 0 {
                        log::warn!(
                            "port {preferred} is in use; the WebSocket server is answering on {port} instead"
                        );
                    }
                    return Ok(listener);
                }
                // A taken address is the only kind worth walking past. Anything else — no
                // permission for the address, a host that does not resolve — fails identically
                // on every port, so a search would turn one clear error into a slower vague one.
                Err(cause) if cause.kind() == std::io::ErrorKind::AddrInUse => taken = Some(cause),
                Err(cause) => return Err(cause.into()),
            }
        }

        let reason = taken
            .map(|cause| cause.to_string())
            .unwrap_or_else(|| "the port range ends at 65535".to_string());

        Err(anyhow::anyhow!(
            "no free port for the WebSocket server between {preferred} and {}: {reason}",
            preferred.saturating_add(span)
        ))
    }
}
