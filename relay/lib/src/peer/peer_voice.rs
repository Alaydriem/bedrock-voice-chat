use common::structs::relay::wire::datagram::{GroupScope, VoiceFrame};

// A voice frame received from a peer, with the linked group it claims, if any.
#[derive(Debug, Clone)]
pub struct PeerVoice {
    pub frame: VoiceFrame,
    pub group: Option<GroupScope>,
}
