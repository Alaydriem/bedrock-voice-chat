// Extracts the subject Common Name from a DER-encoded X.509 certificate.
//
// The input arrives from the network (a peer's presented chain), so every failure
// mode — unparseable DER, no CN attribute, a non-UTF-8 CN — returns `None` rather
// than panicking. The caller treats `None` as "no authenticated identity".
pub struct CertificateCommonName;

impl CertificateCommonName {
    pub fn from_der(der: &[u8]) -> Option<String> {
        let (_, cert) = x509_parser::parse_x509_certificate(der).ok()?;
        let cn = cert.subject().iter_common_name().next()?;
        cn.as_str().ok().map(|s| s.to_owned())
    }
}
