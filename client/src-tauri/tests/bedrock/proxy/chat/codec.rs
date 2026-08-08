use bvc_client_lib::bedrock::{BvcpCodec, ChatCodec};
use common::bedrock_protocol::protocol::packets::generated::misc::text::TextPacket;
use common::bedrock_protocol::protocol::types::generated::{
    AuthorAndMessage, MessageAndParams, MessageOnly, TextPacketBody, TextPacketType,
};

fn authored(kind: TextPacketType, message: &str) -> TextPacket {
    TextPacket {
        localize: false,
        body: TextPacketBody::AuthorAndMessage(AuthorAndMessage {
            message_type: kind,
            player_name: "Petra".to_string(),
            message: message.to_string(),
        }),
        sender_s_xuid: "xuid".to_string(),
        platform_id: String::new(),
        filtered_message: None,
    }
}

fn message_only(kind: TextPacketType, message: &str) -> TextPacket {
    TextPacket {
        localize: false,
        body: TextPacketBody::MessageOnly(MessageOnly {
            message_type: kind,
            message: message.to_string(),
        }),
        sender_s_xuid: String::new(),
        platform_id: String::new(),
        filtered_message: None,
    }
}

#[test]
fn player_chat_decodes_with_its_author() {
    let line = ChatCodec::decode(&authored(TextPacketType::Chat, "anyone got spare iron"))
        .expect("player chat should decode");

    assert_eq!(line.author.as_deref(), Some("Petra"));
    assert_eq!(line.text, "anyone got spare iron");
    assert!(!line.system);
}

#[test]
fn a_server_announcement_decodes_as_a_system_line_with_no_author() {
    let line = ChatCodec::decode(&message_only(TextPacketType::Announcement, "restart in 5"))
        .expect("announcements should decode");

    assert_eq!(line.author, None);
    assert!(line.system);
}

#[test]
fn a_system_message_decodes_as_a_system_line() {
    let line = ChatCodec::decode(&message_only(TextPacketType::SystemMessage, "world saved"))
        .expect("system messages should decode");
    assert!(line.system);
}

// Whispers are addressed to one player. Relaying them into a shared log would show a private
// message to everyone reading chat in the app.
#[test]
fn whispers_are_dropped() {
    assert!(ChatCodec::decode(&authored(TextPacketType::Whisper, "psst")).is_none());
}

#[test]
fn ui_text_is_dropped() {
    for kind in [
        TextPacketType::Tip,
        TextPacketType::Popup,
        TextPacketType::JukeboxPopup,
    ] {
        assert!(
            ChatCodec::decode(&message_only(kind, "hud noise")).is_none(),
            "{kind:?} should be dropped"
        );
    }
}

fn translated(key: &str, params: &[&str]) -> TextPacket {
    TextPacket {
        localize: true,
        body: TextPacketBody::MessageAndParams(MessageAndParams {
            message_type: TextPacketType::Translate,
            message: key.to_string(),
            parameter_list: params.iter().map(|s| s.to_string()).collect(),
        }),
        sender_s_xuid: String::new(),
        platform_id: String::new(),
        filtered_message: None,
    }
}

// `/say`, joins, leaves and deaths all arrive as translation keys rather than text. Dropping
// them made every server event invisible in the app.
#[test]
fn a_server_say_decodes_as_a_system_line() {
    let line = ChatCodec::decode(&translated("chat.type.announcement", &["Server", "hello"]))
        .expect("a /say must decode");
    assert!(line.system);
    assert_eq!(line.text, "[Server] hello");
}

#[test]
fn a_join_decodes_as_a_system_line() {
    let line = ChatCodec::decode(&translated("multiplayer.player.joined", &["Petra"]))
        .expect("a join must decode");
    assert!(line.system);
    assert_eq!(line.text, "Petra joined the game");
}

#[test]
fn a_death_decodes_as_a_system_line() {
    let line = ChatCodec::decode(&translated("death.attack.mob", &["Moth", "Enderman"]))
        .expect("a death must decode");
    assert!(line.system);
    assert_eq!(line.text, "Moth was slain by Enderman");
}

// Achievements, command feedback and UI strings are also Translate. Chat is for what people
// say and what happens to them, not for every string the game emits.
#[test]
fn an_unrelated_translation_key_is_dropped() {
    assert!(ChatCodec::decode(&translated("commands.gamemode.success", &["x"])).is_none());
}

// Servers colour their broadcasts and end them with a `§r` reset. Left in, that renders as
// literal noise on the end of every line.
#[test]
fn formatting_codes_are_stripped_from_text_and_author() {
    let line = ChatCodec::decode(&authored(TextPacketType::Chat, "§ahello world§r"))
        .expect("coloured chat must decode");
    assert_eq!(line.text, "hello world");
    assert!(!line.text.contains('§'));
}

#[test]
fn a_lone_trailing_escape_does_not_survive() {
    let line =
        ChatCodec::decode(&authored(TextPacketType::Chat, "hello§")).expect("must decode");
    assert_eq!(line.text, "hello");
}

#[test]
fn unrecognized_wire_values_are_dropped() {
    assert!(ChatCodec::decode(&message_only(TextPacketType::Unrecognized(99), "?")).is_none());
}

// The four rides below are BVC's own silent chat bus. A leaked ride would broadcast a player's
// audio state into visible chat, so this is a security regression guard rather than tidiness.
#[test]
fn the_bvcp_presence_ride_is_rejected() {
    let ride = BvcpCodec::format_bvcp("tok-1");
    assert!(ChatCodec::decode(&authored(TextPacketType::Chat, &ride)).is_none());
}

#[test]
fn the_bvca_announce_ride_is_rejected() {
    let ride = BvcpCodec::format_bvca("peer.example:443");
    assert!(ChatCodec::decode(&authored(TextPacketType::Chat, &ride)).is_none());
}

#[test]
fn the_bvce_eject_ride_is_rejected() {
    assert!(ChatCodec::decode(&authored(TextPacketType::Chat, "!bvce 10 64 -20")).is_none());
}

#[test]
fn the_bvcs_state_ride_is_rejected() {
    assert!(
        ChatCodec::decode(&authored(
            TextPacketType::Chat,
            "!bvcs:1:q:m=1;d=0;r=0;g=-"
        ))
        .is_none()
    );
}

// The filter anchors at the start. A player quoting a prefix mid-sentence is having a normal
// conversation and must not be silently censored.
#[test]
fn a_ride_prefix_quoted_mid_message_is_still_ordinary_chat() {
    let line = ChatCodec::decode(&authored(TextPacketType::Chat, "look at this !bvcp thing"))
        .expect("a quoted prefix is not a ride");
    assert_eq!(line.text, "look at this !bvcp thing");
}
