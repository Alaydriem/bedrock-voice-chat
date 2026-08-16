// Failures encoding or decoding a peer-link frame.
//
// Encode and Decode both carry a postcard error but are kept distinct: one means
// this server produced something unrepresentable, the other means a peer sent
// something unreadable, and only the second is a remote party's fault.
#[derive(Debug, thiserror::Error)]
pub enum PeerWireError {
    #[error("encoding a peer frame failed: {0}")]
    Encode(postcard::Error),

    #[error("decoding a peer frame failed: {0}")]
    Decode(postcard::Error),

    #[error("datagram of {size} bytes exceeds the {limit} byte limit")]
    TooLarge { size: usize, limit: usize },
}
