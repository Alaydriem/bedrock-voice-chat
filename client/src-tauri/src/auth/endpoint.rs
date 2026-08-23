/// Joins a saved server URL to an API path with exactly one separator between them.
///
/// Both sides arrive unnormalized. A server URL is whatever the user typed and routinely carries a
/// trailing slash; the endpoint constants disagreed about a leading one, which is how every
/// Minecraft auth request came to be sent to `//api/auth/minecraft`.
pub struct ServerEndpoint;

impl ServerEndpoint {
    pub fn join(server: &str, path: &str) -> String {
        format!(
            "{}/{}",
            server.trim_end_matches('/'),
            path.trim_start_matches('/')
        )
    }
}
