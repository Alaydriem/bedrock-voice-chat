/// The `config/bvc.hcl` written on a fresh install: play mode, so a new operator on a hosting
/// panel configures nothing.
pub struct PlayDefaults;

impl PlayDefaults {
    pub const PORT_VARIABLE: &'static str = "BVC_PORT";
    pub const ADDRESS_VARIABLE: &'static str = "BVC_PUBLIC_ADDRESS";
    // The server's own default for `server.port`.
    pub const DEFAULT_PORT: u16 = 443;

    /// The port and optional public address from `BVC_PORT` and `BVC_PUBLIC_ADDRESS`.
    pub fn from_env() -> (u16, Option<String>) {
        Self::from_values(
            std::env::var(Self::PORT_VARIABLE).ok().as_deref(),
            std::env::var(Self::ADDRESS_VARIABLE).ok().as_deref(),
        )
    }

    /// A warning for a `BVC_PORT` that is set but unusable, naming the port used instead.
    /// Unset and blank are quiet.
    pub fn port_warning(raw: Option<&str>) -> Option<String> {
        let raw = raw.map(str::trim).filter(|raw| !raw.is_empty())?;
        match raw.parse::<u16>() {
            Ok(port) if port != 0 => None,
            _ => Some(format!(
                "{} = \"{raw}\" is not a usable port; using {}",
                Self::PORT_VARIABLE,
                Self::DEFAULT_PORT
            )),
        }
    }

    /// An unset, blank or unparsable port is the server default. A blank address is no address.
    pub fn from_values(port: Option<&str>, address: Option<&str>) -> (u16, Option<String>) {
        let port = port
            .and_then(|port| port.trim().parse::<u16>().ok())
            .filter(|port| *port != 0)
            .unwrap_or(Self::DEFAULT_PORT);
        let address = address
            .map(str::trim)
            .filter(|address| !address.is_empty())
            .map(str::to_string);
        (port, address)
    }

    pub fn render(port: u16, address: Option<&str>) -> String {
        let mut hcl = format!("server {{\n  port = {port}\n  mod_loopback = true\n  tls {{\n");
        match address {
            Some(address) => hcl.push_str(&format!(
                "    play {{\n      address = \"{}\"\n    }}\n",
                Self::escape(address)
            )),
            None => hcl.push_str("    play {}\n"),
        }
        hcl.push_str("  }\n}\n");
        hcl
    }

    // Keeps an operator-supplied value inside its HCL string literal.
    fn escape(value: &str) -> String {
        value
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace("${", "$${")
            .replace("%{", "%%{")
    }
}
