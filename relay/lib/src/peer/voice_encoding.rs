use common::structs::relay::wire::datagram::{GroupScope, VoiceFrame};
use common::structs::relay::wire::{Datagram, WireVersion};

// Chooses the datagram a voice frame travels as, for the version a link negotiated.
pub struct VoiceEncoding;

impl VoiceEncoding {
    // `None` means the frame must not be sent on this link at all.
    //
    // A version 1 peer cannot read a group marker. An OPEN group's speech is also heard nearby,
    // so it goes as plain speech, as it did before the marker existed. Members-only speech is
    // withheld: the peer would route it by proximity.
    pub fn encode(
        version: WireVersion,
        frame: VoiceFrame,
        group: Option<GroupScope>,
    ) -> Option<Datagram> {
        match group {
            None => Some(Datagram::Voice(frame)),
            Some(group) if version >= WireVersion::GROUP_SCOPE => {
                Some(Datagram::GroupVoice { frame, group })
            }
            Some(group) if group.members_only => None,
            Some(_) => Some(Datagram::Voice(frame)),
        }
    }
}
