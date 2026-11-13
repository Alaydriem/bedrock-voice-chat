/// Replacement nameservers for development, so a client can resolve play names against a
/// registry on the same machine. Exists only in a debug build that enables the `dev-trust`
/// feature.
pub struct DevPlayNameservers;

impl DevPlayNameservers {
    /// The environment variable holding a comma-separated list of `host:port` nameservers,
    /// read at run time.
    pub const ENV: &'static str = "BVC_DEV_PLAY_NAMESERVERS";

    // Port assumed for an entry that names none.
    const DEFAULT_PORT: u16 = 53;

    /// The nameservers [`Self::ENV`] names, or `None` when it is unset or lists nothing.
    pub fn from_env() -> Option<Vec<String>> {
        let nameservers = Self::from_value(std::env::var(Self::ENV).ok().as_deref())?;
        curia::info!("play zone nameservers replaced for development", {
            "nameservers": nameservers.join(",")
        });

        Some(nameservers)
    }

    /// The nameservers listed in `value`. Blank entries are ignored, and an entry without a
    /// port gets port 53. `None` when nothing is left.
    pub fn from_value(value: Option<&str>) -> Option<Vec<String>> {
        let nameservers: Vec<String> = value?
            .split(',')
            .map(str::trim)
            .filter(|entry| !entry.is_empty())
            .map(Self::with_port)
            .collect();

        if nameservers.is_empty() {
            None
        } else {
            Some(nameservers)
        }
    }

    fn with_port(entry: &str) -> String {
        // A bracketed IPv6 literal carries its port after the closing bracket.
        let has_port = match entry.rsplit_once(':') {
            Some((host, port)) => {
                port.parse::<u16>().is_ok() && (!host.contains(':') || host.ends_with(']'))
            }
            None => false,
        };
        if has_port {
            entry.to_string()
        } else if entry.contains(':') && !entry.starts_with('[') {
            format!("[{entry}]:{}", Self::DEFAULT_PORT)
        } else {
            format!("{entry}:{}", Self::DEFAULT_PORT)
        }
    }
}
