use common::bedrock_protocol::protocol::packets::generated::misc::text::TextPacket;
use common::bedrock_protocol::protocol::types::generated::{TextPacketBody, TextPacketType};
use common::structs::control::bvcs_codec::BVCS_PREFIX;
use log::debug;

use crate::bedrock::proxy::chat::{ChatLine, MinecraftTranslation};
use crate::bedrock::proxy::presence::BvcpCodec;

/// Turns a clientbound `TextPacket` into a chat line, or rejects it.
pub struct ChatCodec;

impl ChatCodec {
    /// Jukebox eject rides, injected serverbound by the proxy and echoed back by the realm.
    const EJECT_PREFIX: &'static str = "!bvce ";

    /// Minecraft's formatting escape. `§` plus one character sets a colour or style, and
    /// servers pepper it through broadcasts — including a trailing `§r` reset, which is what
    /// shows up as garbage at the end of an otherwise clean line.
    const FORMAT_ESCAPE: char = '§';

    /// `None` means the packet is not chat a person should read: a BVC control ride, a private
    /// whisper, HUD text, or a translation key with no rendering.
    pub fn decode(packet: &TextPacket) -> Option<ChatLine> {
        let (kind, author, message, params) = match &packet.body {
            TextPacketBody::AuthorAndMessage(body) => (
                &body.message_type,
                Some(body.player_name.clone()),
                &body.message,
                None,
            ),
            TextPacketBody::MessageOnly(body) => (&body.message_type, None, &body.message, None),
            TextPacketBody::MessageAndParams(body) => (
                &body.message_type,
                None,
                &body.message,
                Some(&body.parameter_list),
            ),
        };

        if Self::is_ride(message) {
            return None;
        }

        match kind {
            TextPacketType::Chat => Some(ChatLine::player(
                Self::strip_formatting(&author.unwrap_or_default()),
                Self::strip_formatting(message),
            )),

            // The server talking rather than a person. No hue, no name.
            TextPacketType::Announcement | TextPacketType::SystemMessage | TextPacketType::Raw => {
                Some(ChatLine::system(Self::strip_formatting(message)))
            }

            // Where `/say`, join, leave and every death message actually live. The game client
            // would localise these; the app is not the game client, so it renders them here.
            TextPacketType::Translate => {
                let empty: Vec<String> = Vec::new();
                let params = params.unwrap_or(&empty);
                match MinecraftTranslation::render(message, params) {
                    Some(text) => Some(ChatLine::system(Self::strip_formatting(&text))),
                    None => {
                        // Named at debug so an event the table does not cover can be added by
                        // reading one log line rather than guessing at Mojang's catalogue.
                        debug!("Bedrock chat: unrendered translation key '{message}' {params:?}");
                        None
                    }
                }
            }

            // Whisper is addressed to one player, and the rest are HUD furniture or JSON text
            // objects that were never meant to be read as chat.
            _ => None,
        }
    }

    /// Removes `§x` pairs. Servers use them for colour and end broadcasts with a `§r` reset,
    /// which otherwise renders as literal noise in the app.
    pub fn strip_formatting(text: &str) -> String {
        let mut out = String::with_capacity(text.len());
        let mut chars = text.chars();

        while let Some(c) = chars.next() {
            if c == Self::FORMAT_ESCAPE {
                // Drop the code that follows. A trailing lone `§` is dropped with it.
                chars.next();
                continue;
            }
            out.push(c);
        }

        out.trim().to_string()
    }

    /// BVC's own silent chat bus. Anchored at the start so a player quoting a prefix in
    /// conversation is not censored.
    ///
    /// This is a security boundary, not tidiness: a leaked `!bvcs:` ride would put a player's
    /// mute, deafen and recording state into visible chat.
    fn is_ride(message: &str) -> bool {
        BvcpCodec::parse_bvcp(message).is_some()
            || BvcpCodec::parse_bvca(message).is_some()
            || message.starts_with(Self::EJECT_PREFIX)
            || message.starts_with(BVCS_PREFIX)
    }
}
