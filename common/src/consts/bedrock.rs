pub const XBOX_CLIENT_ID: &str = "0000000048183522";
// The client proxy's game port. Deliberately outside the Bedrock range so a local
// dedicated server on 19132 cannot collide with it. Discovery does not depend on
// this value: it rides a separate IPv6 multicast socket, and the RakNet
// advertisement carries whichever port is bound.
pub const BEDROCK_LISTEN_PORT: u16 = 28282;

pub const BEDROCK_KEYRING_NS: &str = "bedrock-xbox";
pub const BEDROCK_KEYRING_KEY_REFRESH_TOKEN: &str = "refresh_token";
pub const BEDROCK_KEYRING_KEY_XUID: &str = "xuid";
