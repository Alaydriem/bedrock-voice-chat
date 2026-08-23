use data_encoding::BASE32_NOPAD;
use iroh::EndpointAddr;

use super::ticket_error::PeerTicketError;

// The one value an operator moves between two servers.
//
// A bare public key identifies a peer but does not locate it, so pasting one
// only works where the far side already holds a matching relay URL. A ticket
// carries the key and the paths to it together, which is what lets enrollment
// be a single copy rather than a key plus an out-of-band address.
//
// Lowercase base32 without padding: the value is read aloud, retyped, and
// pasted into config files, so it avoids case ambiguity and characters that
// would need quoting.
pub struct PeerTicket;

impl PeerTicket {
    // Present so a value pasted into the wrong field is refused with a sentence
    // about what it is, rather than a decode failure.
    pub const PREFIX: &'static str = "bvcpeer";

    pub fn mint(addr: &EndpointAddr) -> Result<String, PeerTicketError> {
        let bytes = postcard::to_stdvec(addr).map_err(|_| PeerTicketError::Payload)?;

        Ok(format!(
            "{}{}",
            Self::PREFIX,
            BASE32_NOPAD.encode(&bytes).to_ascii_lowercase()
        ))
    }

    pub fn parse(text: &str) -> Result<EndpointAddr, PeerTicketError> {
        let body = text
            .trim()
            .strip_prefix(Self::PREFIX)
            .ok_or(PeerTicketError::Prefix)?;

        let bytes = BASE32_NOPAD
            .decode(body.to_ascii_uppercase().as_bytes())
            .map_err(|_| PeerTicketError::Alphabet)?;

        postcard::from_bytes(&bytes).map_err(|_| PeerTicketError::Payload)
    }
}
