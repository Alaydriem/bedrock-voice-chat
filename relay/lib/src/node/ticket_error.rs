// Why a string was not a peer ticket.
//
// Separated from a generic parse failure because the three cases send an
// operator to different places: the wrong kind of value, a mangled copy-paste,
// and a ticket that arrived incomplete.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PeerTicketError {
    #[error("a peer link starts with `bvcpeer`; this value does not")]
    Prefix,

    #[error("a peer link carries characters that are not part of one")]
    Alphabet,

    #[error("a peer link decoded, but not into an endpoint; it may be truncated")]
    Payload,
}
