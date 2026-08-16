/// Result of probing the server
pub(super) enum ProbeResult {
    /// Server is available and version is compatible
    Available,
    /// Server is unavailable (network error, timeout, etc.)
    Unavailable,
    /// Server is available but protocol version mismatch
    VersionMismatch {
        client_version: String,
        server_version: String,
        client_too_old: bool,
    },
}
