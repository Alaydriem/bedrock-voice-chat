//! Shared test fixture for /api/admin/* and /api/auth/* integration tests.
//!
//! Each test file declares `mod common;` at the top and gets `TestServer::start()` plus
//! helpers for issuing player certs, signing requests, and asserting responses.
//!
//! The fixture spins up a real Rocket server on a random localhost port with mTLS
//! configured against an in-tempdir CA. No mocks: real TLS, real reqwest, real DB.

#![allow(dead_code)]

pub mod assertions;
pub mod ca;
pub mod fixtures;
pub mod http_client;
pub mod rocket_harness;
pub mod server;

pub use assertions::*;
pub use server::TestServer;
