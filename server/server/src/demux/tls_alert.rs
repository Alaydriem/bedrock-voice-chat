/// A complete TLS alert record, ready to write to a stream that never reached a
/// TLS session.
///
/// The demuxer refuses connections before any handshake completes, so it has no rustls
/// session to send an alert through. A peer that gets a bare TCP close instead cannot
/// tell a refusal from a network fault, and retries. These bytes make the refusal
/// legible: content type `alert` (0x15), TLS 1.2 record version (0x0303) as every
/// implementation still sends for the outermost record, length 2, level `fatal` (0x02),
/// then the description.
pub(crate) struct TlsAlert;

impl TlsAlert {
    pub(crate) const HANDSHAKE_FAILURE: &'static [u8] = &[0x15, 0x03, 0x03, 0x00, 0x02, 0x02, 40];
    pub(crate) const INTERNAL_ERROR: &'static [u8] = &[0x15, 0x03, 0x03, 0x00, 0x02, 0x02, 80];
    /// Sent when the client offered only protocols this server does not serve, which is
    /// how a client learns the WebSocket transport is unavailable here rather than
    /// hanging on a socket that will never answer.
    pub(crate) const NO_APPLICATION_PROTOCOL: &'static [u8] =
        &[0x15, 0x03, 0x03, 0x00, 0x02, 0x02, 120];
}
