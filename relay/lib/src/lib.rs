//! Both ends of a BVC peer link.
//!
//! The transport, and nothing above it. A BVC server consumes this to accept and
//! dial peers; the bridge SDK consumes the same code to dial one server. What
//! differs between them is authorization and what they do with an admitted frame,
//! neither of which lives here.
//!
//! The wire itself — frames, postcard encoding, framing, version negotiation —
//! lives in `common`, because the server and this crate both speak it.

pub mod node;
pub mod peer;
