mod addon_mode;
// Exercises the translation from a `bedrock_protocol::Error`, which only exists when the
// crate is compiled with that dependency. Run with `--features bedrock-protocol`.
#[cfg(feature = "bedrock-protocol")]
mod renewal;
mod world_id;
