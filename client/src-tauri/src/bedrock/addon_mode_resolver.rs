use common::response::ApiConfigBedrockServer;
use common::structs::bedrock::AddonMode;
use log::info;

// Hosts known to block outbound HTTP from a world's addon, so a world reached
// through one cannot be feeding the BVC server itself. A heuristic, not a
// registry: a platform absent from this list still resolves to the default, and
// the per-entry override is what covers it.
const KNOWN_NO_NET_HOSTS: &[&str] = &["aternos.me"];

// Decides which `AddonMode` a direct proxy session runs under. An operator
// declares it per advertised server; a user-saved entry pointed at the same
// address inherits it, so the same world behaves the same way however the user
// reached it.
pub struct AddonModeResolver;

impl AddonModeResolver {
    pub fn proxy(
        explicit: Option<AddonMode>,
        advertised: &[ApiConfigBedrockServer],
        host: &str,
        port: u16,
    ) -> AddonMode {
        let (mode, source) = Self::resolve(explicit, advertised, host, port);

        // Logged because this one flag decides whether the session runs
        // `FullDispatch` or `RelayOnlyDispatch`, and `RelayOnlyDispatch`
        // silently ignores the bvc: sound buses. A world left on `Net` by
        // mistake loses the jukebox and the control plane with no other
        // symptom, so both the verdict and the reason for it are recorded.
        info!(
            "Bedrock: addon mode {mode:?} for {host}:{port} ({source}) — {}",
            Self::consequence(mode)
        );

        mode
    }

    // Kept pure and separate from the logging above so the decision stays
    // straightforward to read and to test.
    fn resolve(
        explicit: Option<AddonMode>,
        advertised: &[ApiConfigBedrockServer],
        host: &str,
        port: u16,
    ) -> (AddonMode, &'static str) {
        if let Some(mode) = explicit {
            return (mode, "set on the server entry");
        }

        if let Some(entry) = advertised
            .iter()
            .find(|entry| entry.port == port && entry.host.eq_ignore_ascii_case(host))
        {
            return (entry.addon_mode, "declared by the advertised server list");
        }

        if Self::is_known_no_net_host(host) {
            return (AddonMode::NoNet, "known no-net host");
        }

        (AddonMode::default(), "default")
    }

    fn consequence(mode: AddonMode) -> &'static str {
        match mode {
            AddonMode::Net => {
                "the addon feeds BVC over HTTP; this proxy relays only and does \
                 not carry the bvc: jukebox or control buses"
            }
            AddonMode::NoNet => "this proxy carries positions, chat and the bvc: buses in-band",
        }
    }

    // Matches the host itself or any subdomain of it. A bare substring test
    // would match `aternos.me.evil.example.com`, which is a different site.
    fn is_known_no_net_host(host: &str) -> bool {
        let host = host.trim_end_matches('.').to_ascii_lowercase();
        KNOWN_NO_NET_HOSTS
            .iter()
            .any(|known| host == *known || host.ends_with(&format!(".{known}")))
    }
}
