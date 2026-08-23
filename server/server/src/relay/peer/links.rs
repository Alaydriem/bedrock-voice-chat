use common::structs::relay::wire::datagram::VoiceFrame;
use dashmap::DashMap;
use iroh::PublicKey;

use bvc_relay::peer::PeerLink;

// Every live peer link, keyed by the node's public key.
//
// The key is the authenticated identity rather than an address, so a peer that
// moves networks is the same entry rather than a new one — which is what made
// `host:port` keying wrong for a design where identity and location are separate.
pub struct PeerLinks {
    links: DashMap<PublicKey, PeerLink>,
}

impl PeerLinks {
    pub fn new() -> Self {
        Self {
            links: DashMap::new(),
        }
    }

    pub fn insert(&self, link: PeerLink) {
        self.links.insert(link.node(), link);
    }

    pub fn remove(&self, node: &PublicKey) {
        self.links.remove(node);
    }

    pub fn len(&self) -> usize {
        self.links.len()
    }

    pub fn is_empty(&self) -> bool {
        self.links.is_empty()
    }

    // Worlds this link carries that a different live link already carries.
    //
    // Keyed on the node so re-establishing a link with the same peer does not
    // report against its own previous entry.
    pub fn worlds_also_carried(&self, link: &PeerLink) -> Vec<String> {
        let node = link.node();

        link.worlds()
            .iter()
            .filter(|world| {
                self.links
                    .iter()
                    .any(|entry| *entry.key() != node && entry.carries_world(world))
            })
            .cloned()
            .collect()
    }

    // Returns how many links took the frame.
    //
    // A send failure counts as not delivered and does not abort the rest: one
    // peer's dead link must not stop the others hearing a speaker.
    pub fn broadcast_world(&self, world: &str, frame: &VoiceFrame) -> usize {
        self.links
            .iter()
            .filter(|entry| entry.carries_world(world))
            .filter(|entry| entry.send(frame.clone()).is_ok())
            .count()
    }
}

impl Default for PeerLinks {
    fn default() -> Self {
        Self::new()
    }
}
