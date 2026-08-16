// Test harness support surfaced from the library so the e2e bin (a separate
// crate root) and any future orchestrator can share the wire protocol and the
// connect helper. The whole module is gated behind the `e2e` feature and is
// never referenced by the production `run()` path.
#![allow(dead_code)]

pub mod bridge;
pub mod connect;
pub mod counters;
pub mod e2e_app_data;
pub mod port_pool;
pub mod signal;

pub use bridge::{Frame, InMsg, OutMsg};
pub use connect::ConnectConfig;
pub use counters::TransportCounters;
pub use e2e_app_data::E2eAppData;
pub use port_pool::PortPool;
pub use signal::Signal;
