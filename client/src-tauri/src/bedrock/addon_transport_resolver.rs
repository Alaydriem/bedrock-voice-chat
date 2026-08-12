use common::response::ApiConfigBedrockServer;
use common::structs::bedrock::AddonTransport;

// Decides which `AddonTransport` a direct proxy session runs under. An operator
// declares it per advertised server; a user-saved entry pointed at the same
// address inherits it, so the same world behaves the same way however the user
// reached it.
pub struct AddonTransportResolver;

impl AddonTransportResolver {
    pub fn proxy(
        explicit: Option<AddonTransport>,
        advertised: &[ApiConfigBedrockServer],
        host: &str,
        port: u16,
    ) -> AddonTransport {
        if let Some(transport) = explicit {
            return transport;
        }

        advertised
            .iter()
            .find(|entry| entry.port == port && entry.host.eq_ignore_ascii_case(host))
            .map(|entry| entry.addon_transport)
            .unwrap_or_default()
    }
}
