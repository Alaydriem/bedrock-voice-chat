// Drives `BedrockSessionState::set_world_uuid_for_test`, which is
// `#[cfg(any(test, feature = "e2e"))]`, so this module compiles solely in the
// e2e configuration.
#[cfg(feature = "e2e")]
mod play_sound;
