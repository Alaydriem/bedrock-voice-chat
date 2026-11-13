pub mod audio;
pub mod bedrock;
pub mod iap;
pub mod variant;
pub mod version;

// The zone every play-zone name sits inside.
pub const PLAY_ZONE_SUFFIX: &str = ".play.bedrockvc.stream";

pub const OPUS_FRAME_DURATION_MS: u32 = 20;
