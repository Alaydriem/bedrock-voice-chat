use super::ClientActionType;

pub const CTL_PREFIX: &str = "bvc:ctl:";

// Decoded `bvc:ctl:` message: either a control action or the panel's scoped
// snapshot request (which is not a ClientAction — it drives the reverse ride).
pub enum CtlMessage {
    Action(ClientActionType),
    Sync { targets: Vec<String> },
}

pub struct CtlCodec;

impl CtlCodec {
    pub fn encode(action: &ClientActionType) -> String {
        match action {
            ClientActionType::SetMuted(on) => format!("{CTL_PREFIX}mute:{}", *on as u8),
            ClientActionType::SetDeafened(on) => format!("{CTL_PREFIX}deafen:{}", *on as u8),
            ClientActionType::SetRecording(on) => format!("{CTL_PREFIX}record:{}", *on as u8),
            ClientActionType::SetVolume { target, volume } => {
                format!("{CTL_PREFIX}vol:{target}:{}", (volume * 100.0).round() as u32)
            }
            ClientActionType::SetHeard { target, muted } => {
                format!("{CTL_PREFIX}hear:{target}:{}", if *muted { 0 } else { 1 })
            }
            ClientActionType::CreateGroup => format!("{CTL_PREFIX}group:create"),
            ClientActionType::JoinGroup { channel } => {
                format!("{CTL_PREFIX}group:join:{channel}")
            }
            ClientActionType::LeaveGroup => format!("{CTL_PREFIX}group:leave"),
        }
    }

    pub fn encode_sync(targets: &[String]) -> String {
        format!("{CTL_PREFIX}sync:{}", targets.join(","))
    }

    pub fn decode(name: &str) -> Option<CtlMessage> {
        let rest = name.strip_prefix(CTL_PREFIX)?;
        let mut it = rest.split(':');
        let msg = match it.next()? {
            "mute" => CtlMessage::Action(ClientActionType::SetMuted(it.next()? == "1")),
            "deafen" => CtlMessage::Action(ClientActionType::SetDeafened(it.next()? == "1")),
            "record" => CtlMessage::Action(ClientActionType::SetRecording(it.next()? == "1")),
            "vol" => {
                let target = it.next()?.to_string();
                let pct: f32 = it.next()?.parse().ok()?;
                CtlMessage::Action(ClientActionType::SetVolume {
                    target,
                    volume: pct / 100.0,
                })
            }
            "hear" => {
                let target = it.next()?.to_string();
                CtlMessage::Action(ClientActionType::SetHeard {
                    target,
                    muted: it.next()? == "0",
                })
            }
            "group" => match it.next()? {
                "create" => CtlMessage::Action(ClientActionType::CreateGroup),
                "join" => CtlMessage::Action(ClientActionType::JoinGroup {
                    channel: it.next()?.to_string(),
                }),
                "leave" => CtlMessage::Action(ClientActionType::LeaveGroup),
                _ => return None,
            },
            "sync" => {
                let targets = match it.next() {
                    Some(s) if !s.is_empty() => s.split(',').map(str::to_string).collect(),
                    _ => Vec::new(),
                };
                CtlMessage::Sync { targets }
            }
            _ => return None,
        };
        Some(msg)
    }
}
