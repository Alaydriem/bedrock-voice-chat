use rocket::request::Request;
use rocket::response::{self, Responder};
use rocket_ws::Channel;

/// A [`Channel`] that echoes an accepted subprotocol back to the client.
///
/// A client that offers subprotocols requires the server to name one it
/// accepts; the WHATWG WebSocket API fails the connection with close code 1002
/// ("Server did not respond with sent protocols") when the handshake response
/// omits the header. Since the ticket travels as a subprotocol, every client
/// offers at least one, so the response must always carry this.
///
/// `rocket_ws` exposes no way to set it -- its `Responder` writes only
/// `Sec-WebSocket-Accept` and `Sec-Websocket-Version` -- so the header is added
/// on top of the response it builds.
pub struct ProtocolChannel<'r> {
    channel: Channel<'r>,
    protocol: &'static str,
}

impl<'r> ProtocolChannel<'r> {
    pub fn new(channel: Channel<'r>, protocol: &'static str) -> Self {
        Self { channel, protocol }
    }
}

impl<'r, 'o: 'r> Responder<'r, 'o> for ProtocolChannel<'o> {
    fn respond_to(self, request: &'r Request<'_>) -> response::Result<'o> {
        let mut response = self.channel.respond_to(request)?;
        // Never the ticket itself: echoing that back would put a credential in
        // a response header for every proxy in the path to log.
        response.set_raw_header("Sec-WebSocket-Protocol", self.protocol);
        Ok(response)
    }
}
