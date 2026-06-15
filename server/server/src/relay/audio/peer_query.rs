use common::structs::relay::{AudioAvailable, RelayEndpoint};
use tokio::sync::oneshot;

// A responder's answer to an outstanding `AudioQuery`, paired with the endpoint
// it arrived from so the fulfiller can HTTP-pull the file from that exact peer.
// The `AudioAvailable` alone carries only the stream token; the responder
// endpoint is the peer link the reply rode in on.
#[derive(Debug, Clone)]
pub struct ResolvedAudio {
    pub available: AudioAvailable,
    pub responder: RelayEndpoint,
}

// Discovery seam the fulfiller's playback path depends on. Broadcasts an
// `AudioQuery` to peers and yields a receiver that resolves with the first
// responder's `AudioAvailable` and endpoint. Abstracted so the playback service
// can be exercised with a stub instead of a live `PeerManager` + peer links.
// The `correlation_id` (the playback `event_id`) keys the outstanding query so
// concurrent fetches of the same `audio_id` never clobber each other.
pub trait AudioPeerQuery: Send + Sync {
    fn query_audio(&self, audio_id: &str, correlation_id: &str) -> oneshot::Receiver<ResolvedAudio>;
}
