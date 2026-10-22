use serde::{Deserialize, Serialize};

/// Every byte one client handed to the server since the last capture was taken, grouped by the
/// boundary it left through, beside the plaintext Opus of each frame it sealed.
///
/// What a scenario asserts the server never sees has to be checked against these: a server can
/// only observe what crossed one of these boundaries.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct WireCapture {
    /// Voice datagrams, whichever transport carried them.
    pub datagrams: Vec<Vec<u8>>,
    /// Frames written to a protected group's key socket.
    pub key_frames: Vec<Vec<u8>>,
    /// HTTP request bodies sent to the server's API.
    pub http_bodies: Vec<Vec<u8>>,
    /// The Opus each sealed frame carried before sealing. Never sent: the reference the sent
    /// bytes are searched for.
    pub sealed_plaintext: Vec<Vec<u8>>,
}
