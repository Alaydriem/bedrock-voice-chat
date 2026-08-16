// Why a peer's voice frame was not admitted.
//
// Each names the frame's speaker rather than only the peer, because a peer
// carries many speakers and "one of them was refused" is not actionable.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum IngestRejection {
    #[error("the speaker carries no relay world, so no grant can cover it")]
    NoWorld,

    #[error("peer is not granted world {world:?} for speaker {speaker:?}")]
    NotGranted { speaker: String, world: String },

    #[error("peer named {speaker:?}, who is a client of this server")]
    ImpersonatesLocalPlayer { speaker: String },
}
