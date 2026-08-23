use rocket::{
    async_trait,
    http::Status,
    request::{FromRequest, Outcome, Request},
};

use crate::stream::quic::{CacheManager, TicketIdentity};

// Tickets are offered as a WebSocket subprotocol rather than a query parameter
// so they stay out of access logs, proxy logs and browser history.
const TICKET_PREFIX: &str = "ticket.";

/// The identity a client's single-use ticket confers, redeemed before the upgrade.
///
/// The browser WebSocket API cannot set request headers, but it can offer
/// subprotocols, which arrive here as `Sec-WebSocket-Protocol`. That is the
/// only channel available for carrying a credential into an upgrade, short of
/// putting it in the URL where it would be logged.
///
/// Redeeming here rather than inside the channel is what makes a refused ticket
/// legible. A guard failure is answered with 401 and no upgrade; redeeming after
/// the upgrade meant the server had already sent `101 Switching Protocols` and
/// could only drop the transport, which a browser reports as a cancelled opening
/// handshake with no status and no reason. The client could not tell a spent
/// ticket from a network fault, so it retried into the same wall indefinitely.
///
/// Because the ticket is offered as a subprotocol, the client always offers at
/// least one, and a client that offers any requires the server to accept one by
/// name -- the WHATWG WebSocket API closes with 1002 otherwise. Routes using
/// this guard must therefore respond through
/// [`ProtocolChannel`](crate::http::routes::api::websocket::protocol_channel::ProtocolChannel),
/// which echoes a feed protocol rather than the ticket.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WebsocketTicket(pub TicketIdentity);

#[async_trait]
impl<'r> FromRequest<'r> for WebsocketTicket {
    type Error = ();

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let ticket = req
            .headers()
            .get("Sec-WebSocket-Protocol")
            .flat_map(|header| header.split(','))
            .map(str::trim)
            .find_map(|protocol| protocol.strip_prefix(TICKET_PREFIX))
            .filter(|ticket| !ticket.is_empty())
            .map(str::to_string);

        let Some(ticket) = ticket else {
            return Outcome::Error((Status::Unauthorized, ()));
        };

        let Some(cache_manager) = req.rocket().state::<CacheManager>() else {
            tracing::error!("websocket upgrade attempted before the cache manager was managed");
            return Outcome::Error((Status::InternalServerError, ()));
        };

        match cache_manager.websocket_tickets().redeem(&ticket).await {
            Some(identity) => Outcome::Success(WebsocketTicket(identity)),
            None => {
                tracing::debug!("websocket upgrade presented an unknown or spent ticket");
                Outcome::Error((Status::Unauthorized, ()))
            }
        }
    }
}
