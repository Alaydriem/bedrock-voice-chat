/// The ALPN token a client offers to reach the WebSocket voice transport.
///
/// Shared by client and server: a disagreement routes voice traffic into the HTTP API.
///
/// Unversioned. Version compatibility is enforced by `PROTOCOL_VERSION` once the session
/// is up, where a mismatch can name itself; an ALPN refusal is a failed handshake with no
/// error channel.
pub struct VoiceProtocol;

impl VoiceProtocol {
    pub const ALPN: &'static [u8] = b"bvc-voice";
}
