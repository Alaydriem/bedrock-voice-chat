/// Filesystem locations of the active ACME certificate material, in the
/// exact shape `tls.certificate` / `tls.key` expect.
#[derive(Debug)]
pub struct AcmeCertPaths {
    pub certificate: String,
    pub key: String,
}
