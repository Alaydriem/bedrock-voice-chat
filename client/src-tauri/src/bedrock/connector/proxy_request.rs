/// What a caller must say to start the proxy against a direct backend.
///
/// `listen_port` and `network_interface` are optional because a caller that is not the
/// settings pane has no opinion about either: a script naming a saved server wants the
/// same defaults a click would have produced, resolved by the connector rather than
/// guessed at the call site.
pub struct ProxyConnectRequest {
    pub target_host: String,
    pub target_port: u16,
    pub listen_port: Option<u16>,
    pub network_interface: Option<String>,
    /// Raw Bedrock protocol version to advertise. `None` mirrors the real backend.
    pub advertised_protocol: Option<u32>,
    /// Declared addon transport for this target. `None` resolves from the advertised
    /// list, then defaults to no-net.
    pub addon_transport: Option<common::structs::bedrock::AddonTransport>,
}
