use super::{PlayerPreference, QueryState};

pub const BVCS_PREFIX: &str = "!bvcs:";

// Decoded `!bvcs:` reverse-ride message. The wire omits the player identity —
// the BDS listener attributes each message to the chat sender, exactly as the
// proxy injected it on that player's own session.
#[derive(Debug, Clone, PartialEq)]
pub enum BvcsMessage {
    QueryState {
        muted: bool,
        deafened: bool,
        recording: bool,
        group: Option<String>,
    },
    Preference {
        target: String,
        volume: f32,
        muted: bool,
    },
    GroupListHeader {
        count: usize,
    },
    Group {
        id: String,
        name: String,
    },
}

/// Encodes/decodes the no-net reverse-ride grammar the proxy injects as
/// serverbound chat and the standalone BDS mod parses
/// (mods/bds/src/state/bvcs_codec.ts — keep both ends in sync; the common-side
/// goldens pin the strings):
///
/// `!bvcs:<seq>:q:m=<0|1>;d=<0|1>;r=<0|1>;g=<group|->` — self-state snapshot
/// `!bvcs:<seq>:p:t=<target>;v=<percent>;h=<0|1>`      — one player preference
///
/// Volume rides as a rounded percent and mute as an inverted heard flag,
/// mirroring the `bvc:ctl:` grammar's conventions. `<seq>` is a monotonic
/// message tag; each message is bounded far below the TextPacket limit, so no
/// chunking path exists.
pub struct BvcsCodec;

impl BvcsCodec {
    pub fn encode_query_state(seq: u64, state: &QueryState) -> String {
        format!(
            "{BVCS_PREFIX}{seq}:q:m={};d={};r={};g={}",
            state.muted as u8,
            state.deafened as u8,
            state.recording as u8,
            state.current_group.as_deref().unwrap_or("-"),
        )
    }

    /// A target is wire-safe when it cannot corrupt the `key=value;` grammar.
    /// Xbox gamertags never contain these characters; a future identity source
    /// that does must not ride until the grammar gains escaping.
    pub fn target_is_wire_safe(target: &str) -> bool {
        !target.contains([';', '=', ':'])
    }

    pub fn encode_preference(seq: u64, preference: &PlayerPreference) -> String {
        format!(
            "{BVCS_PREFIX}{seq}:p:t={};v={};h={}",
            preference.target,
            (preference.volume * 100.0).round() as u32,
            if preference.muted { 0 } else { 1 },
        )
    }

    /// Announces how many `g` rows follow. A receiver clears its list on this message,
    /// so rows from an earlier request that arrive afterwards are discarded rather than
    /// mixed into the new one.
    pub fn encode_group_header(seq: u64, count: usize) -> String {
        format!("{BVCS_PREFIX}{seq}:gl:c={count}")
    }

    pub fn encode_group(seq: u64, id: &str, name: &str) -> String {
        format!("{BVCS_PREFIX}{seq}:g:i={id};n={}", Self::escape(name))
    }

    pub fn decode(message: &str) -> Option<BvcsMessage> {
        let rest = message.strip_prefix(BVCS_PREFIX)?;
        let mut it = rest.splitn(3, ':');
        let _seq: u64 = it.next()?.parse().ok()?;
        let kind = it.next()?;
        let fields = Self::parse_fields(it.next()?);

        match kind {
            "q" => Some(BvcsMessage::QueryState {
                muted: fields.iter().find(|(k, _)| *k == "m")?.1 == "1",
                deafened: fields.iter().find(|(k, _)| *k == "d")?.1 == "1",
                recording: fields.iter().find(|(k, _)| *k == "r")?.1 == "1",
                group: fields
                    .iter()
                    .find(|(k, _)| *k == "g")
                    .and_then(|(_, v)| (*v != "-").then(|| v.to_string())),
            }),
            "p" => {
                let target = fields.iter().find(|(k, _)| *k == "t")?.1.to_string();
                let percent: f32 = fields.iter().find(|(k, _)| *k == "v")?.1.parse().ok()?;
                if !percent.is_finite() {
                    return None;
                }
                let heard = fields.iter().find(|(k, _)| *k == "h")?.1 == "1";
                Some(BvcsMessage::Preference {
                    target,
                    volume: percent / 100.0,
                    muted: !heard,
                })
            }
            "gl" => {
                let count: usize = fields.iter().find(|(k, _)| *k == "c")?.1.parse().ok()?;
                Some(BvcsMessage::GroupListHeader { count })
            }
            "g" => {
                let id = fields.iter().find(|(k, _)| *k == "i")?.1.to_string();
                let name = Self::unescape(fields.iter().find(|(k, _)| *k == "n")?.1)?;
                Some(BvcsMessage::Group { id, name })
            }
            _ => None,
        }
    }

    fn parse_fields(payload: &str) -> Vec<(&str, &str)> {
        payload
            .split(';')
            .filter_map(|kv| kv.split_once('='))
            .collect()
    }

    // The grammar delimits on `;`, `=` and `:`, which a renamed group may contain.
    // `%` is encoded first so decoding is unambiguous.
    fn escape(value: &str) -> String {
        let mut out = String::with_capacity(value.len());
        for c in value.chars() {
            match c {
                '%' => out.push_str("%25"),
                ';' => out.push_str("%3B"),
                '=' => out.push_str("%3D"),
                ':' => out.push_str("%3A"),
                _ => out.push(c),
            }
        }
        out
    }

    fn unescape(value: &str) -> Option<String> {
        let bytes = value.as_bytes();
        let mut out = Vec::with_capacity(bytes.len());
        let mut index = 0;
        while index < bytes.len() {
            if bytes[index] == b'%' {
                let hex = value.get(index + 1..index + 3)?;
                out.push(u8::from_str_radix(hex, 16).ok()?);
                index += 3;
            } else {
                out.push(bytes[index]);
                index += 1;
            }
        }
        String::from_utf8(out).ok()
    }
}
