use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, ServerName};

use crate::errors::PlayZoneError;

/// The play roots compiled into this build, and the rules for which names they may vouch for.
pub struct PlayRoots;

impl PlayRoots {
    pub const SUFFIX: &'static str = ".play.bedrockvc.stream";

    // Every root is listed here by name; a file added to the directory but not to this
    // list is not trusted. Only the development root exists so far, so a release build
    // lists none and refuses every play name.
    #[cfg(debug_assertions)]
    const BUILTIN: &'static [&'static [u8]] =
        &[include_bytes!("../../../resources/play_roots/bvc-play-root-dev.pem")];
    #[cfg(not(debug_assertions))]
    const BUILTIN: &'static [&'static [u8]] = &[];

    /// The roots that ship with the client. An entry that does not parse is an error, not
    /// an omission.
    pub fn builtin() -> Result<Vec<CertificateDer<'static>>, PlayZoneError> {
        Self::BUILTIN
            .iter()
            .flat_map(|pem| CertificateDer::pem_slice_iter(pem))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|source| PlayZoneError::PlayRoot(source.to_string()))
    }

    /// Every certificate in a PEM bundle.
    pub fn parse_pem(pem: &str) -> Result<Vec<CertificateDer<'static>>, PlayZoneError> {
        CertificateDer::pem_slice_iter(pem.as_bytes())
            .collect::<Result<Vec<_>, _>>()
            .map_err(|source| PlayZoneError::ExtraRoot(source.to_string()))
    }

    /// Whether a play root may vouch for this name. Case-insensitive; the zone apex is not
    /// a play name.
    pub fn is_play_name(name: &ServerName<'_>) -> bool {
        match name {
            ServerName::DnsName(dns) => Self::is_zone_name(dns.as_ref()),
            _ => false,
        }
    }

    /// Whether a DNS name sits strictly inside the zone. A trailing dot is ignored, and a
    /// wildcard label is refused: a play certificate names one server.
    pub fn is_zone_name(name: &str) -> bool {
        let name = name.trim_end_matches('.').to_ascii_lowercase();

        name.len() > Self::SUFFIX.len() && name.ends_with(Self::SUFFIX) && !name.contains('*')
    }
}
