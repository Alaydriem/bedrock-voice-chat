use rocket::{
    async_trait,
    http::Status,
    request::{FromRequest, Outcome, Request},
};

// Tickets are offered as a WebSocket subprotocol rather than a query parameter
// so they stay out of access logs, proxy logs and browser history.
const TICKET_PREFIX: &str = "ticket.";

/// The single-use ticket a client offers when opening a WebSocket.
///
/// The browser WebSocket API cannot set request headers, but it can offer
/// subprotocols, which arrive here as `Sec-WebSocket-Protocol`. That is the
/// only channel available for carrying a credential into an upgrade, short of
/// putting it in the URL where it would be logged.
///
/// Because the ticket is offered as a subprotocol, the client always offers at
/// least one, and a client that offers any requires the server to accept one by
/// name -- the WHATWG WebSocket API closes with 1002 otherwise. Routes using
/// this guard must therefore respond through
/// [`ProtocolChannel`](crate::http::routes::api::websocket::protocol_channel::ProtocolChannel),
/// which echoes a feed protocol rather than the ticket.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WebsocketTicket(pub String);

#[async_trait]
impl<'r> FromRequest<'r> for WebsocketTicket {
    type Error = ();

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let ticket = req
            .headers()
            .get("Sec-WebSocket-Protocol")
            .flat_map(|header| header.split(','))
            .map(str::trim)
            .find_map(|protocol| protocol.strip_prefix(TICKET_PREFIX));

        match ticket {
            Some(ticket) if !ticket.is_empty() => {
                Outcome::Success(WebsocketTicket(ticket.to_string()))
            }
            _ => Outcome::Error((Status::Unauthorized, ())),
        }
    }
}
