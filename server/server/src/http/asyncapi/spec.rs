use common::structs::chat::ChatFrame;
use common::structs::position::PositionSnapshot;
use rocket::uri;
use schemars::r#gen::{SchemaGenerator, SchemaSettings};
use serde_json::{Value, json};

use crate::http::routes::api::websocket::{chat, positions, ticket};
use crate::services::position_service::{POSITION_SCOPE_MAX, POSITION_SCOPE_MULTIPLIER};
use crate::services::{FAR_TIER_MAX, PositionFeedService};

// schemars writes every `$ref` against this path, so the definitions it collects drop
// straight into `components.schemas` with no rewriting.
const SCHEMA_PATH: &str = "#/components/schemas/";

const API_REFERENCE: &str = "https://www.bedrockvoicechat.com/api";
const WIKI_GUIDE: &str = "https://www.bedrockvoicechat.com/wiki/reference/server-websocket/";

/// The AsyncAPI document for the server's WebSocket routes.
///
/// The upgrades are mounted outside the OpenAPI spec because a socket has no response
/// schema, so this is the only reference that describes them. Addresses, limits and payloads
/// come from the routes and the `common` types they serialize, so a renamed path, a changed
/// limit or a new field reaches the published document without a hand edit.
pub struct AsyncApiSpec;

impl AsyncApiSpec {
    pub fn generate() -> Value {
        let mut generator = SchemaGenerator::new(
            SchemaSettings::draft07().with(|settings| {
                settings.definitions_path = SCHEMA_PATH.to_string();
            }),
        );
        let position_payload = generator.subschema_for::<PositionSnapshot>();
        let chat_payload = generator.subschema_for::<ChatFrame>();

        json!({
            "asyncapi": "3.0.0",
            "info": {
                "title": "Bedrock Voice Chat Server WebSocket API",
                "version": env!("CARGO_PKG_VERSION"),
                "description": Self::overview()
            },
            "servers": {
                "server": {
                    "host": "{host}",
                    "protocol": "wss",
                    "description": "Your BVC server. Use the same host and port as its HTTP API.",
                    "variables": {
                        "host": {
                            "description": "The server's host name and HTTPS port, for example voice.example.com:443."
                        }
                    }
                }
            },
            "channels": Self::channels(),
            "operations": Self::operations(),
            "components": {
                "messages": Self::messages(position_payload, chat_payload),
                "schemas": generator.take_definitions(),
                "securitySchemes": Self::security_schemes()
            }
        })
    }

    fn overview() -> String {
        let positions = uri!("/api", positions::positions);
        let chat = uri!("/api", chat::chat);

        format!(
            "The BVC server has two WebSocket feeds.\n\n\
             | Feed | Address | Used by |\n\
             |---|---|---|\n\
             | Position feed | `{positions}` | The BVC app, to show who is near the player |\n\
             | Chat bridge | `{chat}` | The BVC Addon and mods, to pass chat between the game and the app |\n\n\
             Both use `wss://` on the same host and port as the [HTTP API]({API_REFERENCE}).\n\n\
             A step-by-step guide with examples is on the wiki: [Server WebSocket]({WIKI_GUIDE})."
        )
    }

    fn channels() -> Value {
        json!({
            "positions": {
                "address": uri!("/api", positions::positions).to_string(),
                "title": "Position feed",
                "description": Self::positions_description(),
                "messages": {
                    "positionSnapshot": { "$ref": "#/components/messages/PositionSnapshot" }
                }
            },
            "chat": {
                "address": uri!("/api", chat::chat).to_string(),
                "title": "Chat bridge",
                "description": Self::chat_description(),
                "messages": {
                    "chatFrame": { "$ref": "#/components/messages/ChatFrame" }
                }
            }
        })
    }

    fn positions_description() -> String {
        let ticket = uri!("/api", ticket::ticket);
        let interval_ms = PositionFeedService::TICK.as_millis();
        let ticket_seconds = ticket::TICKET_EXPIRES_IN;
        let protocol = positions::PROTOCOL;
        let session_hours = positions::SESSION_MAX.as_secs() / 3600;

        format!(
            "Tells a player who is near them in game, and where. The server sends a snapshot every {interval_ms} ms.\n\n\
             ### How to connect\n\n\
             1. Call `POST {ticket}` with the player's BVC client certificate. The response contains a `ticket`.\n\
             2. Within {ticket_seconds} seconds, open the socket. Offer two subprotocols: `ticket.<ticket>` and `{protocol}`.\n\
             3. Read the messages. Each one is a JSON PositionSnapshot.\n\n\
             ### Who is in a snapshot\n\n\
             - Every player within voice range of you. Voice range is `voice.spatial_audio.broadcast_range` in the server's configuration.\n\
             - Up to {FAR_TIER_MAX} more players beyond voice range, nearest first. The feed reaches {POSITION_SCOPE_MULTIPLIER} times voice range, and never more than {POSITION_SCOPE_MAX} blocks.\n\
             - Only players in the same world and dimension as you.\n\
             - Players who do not have BVC. Their `presence` is `game`.\n\n\
             Positions are relative to you. The feed never sends coordinates.\n\n\
             ### Limitations\n\n\
             - You need the player's client certificate to get a ticket. The BVC app receives this certificate when the player signs in. Other tools cannot get one.\n\
             - Each ticket works once. Asking for a new ticket cancels the previous one.\n\
             - The server closes the socket after {session_hours} hours. Get a new ticket and connect again.\n\
             - If you are not in the game, each snapshot has an empty `positions` list. The socket stays open.\n\
             - The server ignores messages you send on this socket."
        )
    }

    fn chat_description() -> String {
        let queue = chat::OUTBOUND_CAPACITY;

        format!(
            "Carries chat between a game server and the BVC app. Chat typed in game appears in the app. Chat typed in the app appears in game. The BVC Addon and the BVC mods use this socket.\n\n\
             ### How to connect\n\n\
             1. Open the socket with the header `Authorization: Bearer <token>`. The token is `server.minecraft.access_token` in the server's configuration.\n\
             2. Do not offer a subprotocol. The handshake fails if you do.\n\
             3. Send a `hello` message. It must be the first message.\n\
             4. Send `chat` and `event` messages as things happen in game. Show each `say` message you receive in game chat.\n\n\
             ### Messages\n\n\
             | `t` | Direction | Meaning |\n\
             |---|---|---|\n\
             | `hello` | Game to server | Names the world. Send it first, once. |\n\
             | `chat` | Game to server | A player typed a message in game. |\n\
             | `event` | Game to server | A line from the game itself, such as a death or a join. |\n\
             | `say` | Server to game | A player typed a message in the app. |\n\n\
             ### Limitations\n\n\
             - Use one socket for each world. A new socket for the same world closes the old one.\n\
             - A `chat` or `event` message sent before `hello` closes the socket. A second `hello` is ignored.\n\
             - If `server.features.chat` is `false`, the socket connects but no chat goes through in either direction.\n\
             - Chat is not stored. Messages sent while the socket is closed are lost.\n\
             - Read `say` messages promptly. The server holds up to {queue} unread messages and drops the rest."
        )
    }

    fn operations() -> Value {
        json!({
            "sendPositions": {
                "action": "send",
                "channel": { "$ref": "#/channels/positions" },
                "title": "Server sends a position snapshot",
                "security": [{ "$ref": "#/components/securitySchemes/websocketTicket" }],
                "messages": [{ "$ref": "#/channels/positions/messages/positionSnapshot" }]
            },
            "receiveChat": {
                "action": "receive",
                "channel": { "$ref": "#/channels/chat" },
                "title": "Game sends hello, chat and event messages",
                "security": [{ "$ref": "#/components/securitySchemes/accessToken" }],
                "messages": [{ "$ref": "#/channels/chat/messages/chatFrame" }]
            },
            "sendChat": {
                "action": "send",
                "channel": { "$ref": "#/channels/chat" },
                "title": "Server sends say messages",
                "security": [{ "$ref": "#/components/securitySchemes/accessToken" }],
                "messages": [{ "$ref": "#/channels/chat/messages/chatFrame" }]
            }
        })
    }

    fn messages(
        position_payload: impl serde::Serialize,
        chat_payload: impl serde::Serialize,
    ) -> Value {
        json!({
            "PositionSnapshot": {
                "name": "PositionSnapshot",
                "title": "Position snapshot",
                "description": "A JSON text message listing everyone near the player.",
                "contentType": "application/json",
                "payload": position_payload
            },
            "ChatFrame": {
                "name": "ChatFrame",
                "title": "Chat message",
                "description": "A JSON text message. The `t` field says which kind it is.",
                "contentType": "application/json",
                "payload": chat_payload
            }
        })
    }

    fn security_schemes() -> Value {
        let ticket = uri!("/api", ticket::ticket);
        let protocol = positions::PROTOCOL;

        json!({
            "websocketTicket": {
                "type": "httpApiKey",
                "name": "Sec-WebSocket-Protocol",
                "in": "header",
                "description": format!(
                    "A one-time ticket from `POST {ticket}`, sent as the subprotocol `ticket.<ticket>` together with `{protocol}`. A missing or used ticket gets HTTP 401."
                )
            },
            "accessToken": {
                "type": "http",
                "scheme": "bearer",
                "description": "The server's access token, `server.minecraft.access_token`."
            }
        })
    }
}
