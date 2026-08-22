use common::structs::packet::SpeakerPosition;

/// What a listener knows about one speaker between position heartbeats.
///
/// `name` is what the speaker is keyed and displayed as: a player's canonical identity
/// rendered, or the service name for injected audio. A `String` rather than a
/// `PlayerIdentity` because a jukebox and the channel API are legitimately speakers here and
/// neither is a player.
#[derive(Clone, Debug)]
pub struct SpeakerState {
    pub name: String,
    /// Absent until a frame carrying a position has arrived for this speaker. Presence and
    /// gain still work without it; spatial panning does not.
    pub speaker: Option<SpeakerPosition>,
}
