use common::structs::relay::wire::control::RefuseReason;

#[derive(Debug, thiserror::Error)]
pub enum PeerError {
    #[error("binding the peer endpoint: {0}")]
    Bind(String),

    #[error("the peer link failed: {0}")]
    Transport(String),

    #[error("the peer wire rejected a frame: {0}")]
    Wire(#[from] common::errors::PeerWireError),

    #[error("the peer refused the link: {0:?}")]
    Refused(RefuseReason),

    #[error("expected {expected} on the control stream, got something else")]
    Unexpected { expected: &'static str },

    #[error("the link negotiated {negotiated} datagram bytes; BVC frames need {required}")]
    DatagramTooSmall { negotiated: usize, required: usize },
}
