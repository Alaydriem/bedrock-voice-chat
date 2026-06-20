//! DB-direct insert helpers, used to seed the fixture before each test.
//!
//! These mirror the production paths (`PlayerRegistrarService::create_player`,
//! `PermissionService::set_override`) but bypass the HTTP layer because the tests
//! are exercising the routes themselves and need a known starting state.

pub mod permission;
pub mod player;

pub use permission::PermissionFixture;
pub use player::PlayerFixture;
