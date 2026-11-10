use rustls::pki_types::CertificateDer;
use rustls::pki_types::pem::PemObject;

// The play roots that clients of this build pin. The registry cannot link the client's
// list, so it reads the same files.
pub struct PlayRootPin;

impl PlayRootPin {
    const PRODUCTION: &'static [u8] =
        include_bytes!("../../../../common/resources/play_roots/bvc-play-root-2026.pem");

    #[cfg(debug_assertions)]
    const PINNED: &'static [&'static [u8]] = &[
        Self::PRODUCTION,
        include_bytes!("../../../../common/resources/play_roots/bvc-play-root-dev.pem"),
    ];
    #[cfg(not(debug_assertions))]
    const PINNED: &'static [&'static [u8]] = &[Self::PRODUCTION];

    // True when `root_pem` holds at least one certificate and every one is pinned. A root
    // that is not pinned signs names that no release client accepts.
    pub fn is_pinned(root_pem: &str) -> Result<bool, rustls::pki_types::pem::Error> {
        let given = CertificateDer::pem_slice_iter(root_pem.as_bytes())
            .collect::<Result<Vec<_>, _>>()?;
        let mut pinned = Vec::new();
        for pem in Self::PINNED {
            for certificate in CertificateDer::pem_slice_iter(pem) {
                pinned.push(certificate?);
            }
        }

        Ok(!given.is_empty() && given.iter().all(|c| pinned.contains(c)))
    }
}
