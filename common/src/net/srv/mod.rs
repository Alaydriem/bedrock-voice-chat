mod authoritative_query;
#[cfg(all(debug_assertions, feature = "dev-trust"))]
mod dev_play_nameservers;
mod doh_lookup;
mod play_zone_dns;
mod record_choice;
mod srv_answer;
mod srv_fallback;
mod srv_lookup;
mod srv_resolution;
mod srv_resolver;
mod srv_resolver_config;
mod srv_wire;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
mod system_lookup;

pub(crate) use authoritative_query::AuthoritativeQuery;
#[cfg(all(debug_assertions, feature = "dev-trust"))]
pub use dev_play_nameservers::DevPlayNameservers;
pub(crate) use doh_lookup::DohLookup;
pub use play_zone_dns::PlayZoneDns;
pub use record_choice::SrvRecordChoice;
pub use srv_answer::SrvAnswer;
pub use srv_fallback::SrvFallback;
pub use srv_lookup::SrvLookup;
pub use srv_resolution::SrvResolution;
pub use srv_resolver::SrvResolver;
pub use srv_resolver_config::SrvResolverConfig;
pub(crate) use srv_wire::SrvWire;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub(crate) use system_lookup::SystemLookup;
