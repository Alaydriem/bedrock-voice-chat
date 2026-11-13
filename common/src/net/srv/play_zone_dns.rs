/// Where BVC service records are looked up.
pub struct PlayZoneDns;

impl PlayZoneDns {
    // The owner name of a record is `<SERVICE>.<host>`.
    pub const SERVICE: &'static str = "_bvc._tcp";

    // The play zone's authoritative nameservers, asked directly.
    pub const NAMESERVERS: &'static [&'static str] = &["ns1.bedrockvc.stream:53"];

    pub const DOH_URL: &'static str = "https://cloudflare-dns.com/dns-query";
}
