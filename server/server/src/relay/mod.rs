//! This server's half of a peer link.
//!
//! `bvc-relay` carries the transport — endpoint, handshake, link, admission. What
//! lives here is everything a *server* adds on top of it: who may peer and for
//! which worlds, what an inbound frame must satisfy before it reaches local
//! clients, and which local audio leaves at all.

pub mod grant;
pub mod peer;
pub mod world;

pub use grant::{Grant, GrantConfigError, GrantTable};
pub use peer::{
    AdvertisedAddress, IngestRejection, LocalClients, PeerBlock, PeerEgress, PeerIngest, PeerLinks, PeerPlane, PeerSink,
};
pub use world::{RelayWorldWatch, WorldWatchState};
