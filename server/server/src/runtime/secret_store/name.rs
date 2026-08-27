/// The name of a row in `server_secret`.
///
/// An enum rather than a string literal at each call site: a typo would silently create a
/// second row and resolve to a freshly generated value, which for the node key means a new
/// peer identity and for the access token means every mod losing its credential.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretName {
    MinecraftAccessToken,
    RelayNodeKey,
    RelayAssignedName,
}

impl SecretName {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::MinecraftAccessToken => "minecraft_access_token",
            Self::RelayNodeKey => "relay_node_key",
            Self::RelayAssignedName => "relay_assigned_name",
        }
    }
}
