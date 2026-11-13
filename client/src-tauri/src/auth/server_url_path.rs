/// Joins a server base URL to an API path with exactly one separator between them.
///
/// Both sides arrive unnormalized. A base URL routinely carries a trailing slash; the endpoint
/// constants disagreed about a leading one, which is how every Minecraft auth request came to
/// be sent to `//api/auth/minecraft`.
pub struct ServerUrlPath;

impl ServerUrlPath {
    pub fn join(base: &str, path: &str) -> String {
        format!(
            "{}/{}",
            base.trim_end_matches('/'),
            path.trim_start_matches('/')
        )
    }
}
