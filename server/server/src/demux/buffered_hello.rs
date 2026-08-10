/// A ClientHello the demuxer has read off the wire, plus the routing signal inside it.
pub(crate) struct BufferedHello {
    /// Every byte consumed from the client, replayed to the backend so the handshake
    /// continues untouched. Read rather than peeked, so nothing is left unread in the
    /// receive queue to turn a close into an RST.
    pub bytes: Vec<u8>,
    /// The ALPN protocols the client offered. Empty when it offered none, which is the
    /// ordinary case for a browser and must route to the API rather than be refused.
    pub alpn: Vec<Vec<u8>>,
}

impl BufferedHello {
    pub(crate) fn offers(&self, protocol: &[u8]) -> bool {
        self.alpn.iter().any(|offered| offered == protocol)
    }
}
