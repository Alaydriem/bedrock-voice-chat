use rcgen::{
    BasicConstraints, CertificateParams, CertificateSigningRequestParams, CertifiedIssuer,
    DistinguishedName, DnType, ExtendedKeyUsagePurpose, GeneralSubtree, IsCa, KeyPair,
    KeyUsagePurpose, NameConstraints,
};

use super::error::PlayError;

// An in-process CA for tests and local development, standing in for step-ca.
//
// Name-constrained to the play zone like the real intermediate, and it signs 360 hour
// leaves. Never used in a deployment: its key lives and dies with the process.
pub struct LocalSigner {
    issuer: CertifiedIssuer<'static, KeyPair>,
}

impl LocalSigner {
    const ZONE: &'static str = "play.bedrockvc.stream";
    const LEAF_LIFETIME: time::Duration = time::Duration::hours(360);

    pub fn new() -> Self {
        let mut params = CertificateParams::default();
        let mut name = DistinguishedName::new();
        name.push(DnType::CommonName, "BVC play local test CA");
        params.distinguished_name = name;
        params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        params.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];
        params.name_constraints = Some(NameConstraints {
            permitted_subtrees: vec![GeneralSubtree::DnsName(Self::ZONE.to_string())],
            excluded_subtrees: Vec::new(),
        });

        let key = KeyPair::generate().expect("a signing key generates");
        let issuer = CertifiedIssuer::self_signed(params, key).expect("the CA certificate signs");

        Self { issuer }
    }

    pub fn root_pem(&self) -> String {
        self.issuer.pem()
    }

    // The leaf followed by the CA certificate. The caller has already checked the
    // request's names; the leaf takes its names and key from the CSR.
    pub fn sign(&self, csr_pem: &str, _fqdn: &str) -> Result<String, PlayError> {
        let mut request = CertificateSigningRequestParams::from_pem(csr_pem)
            .map_err(|e| PlayError::CsrRejected(format!("the request does not parse: {e}")))?;

        let now = time::OffsetDateTime::now_utc();
        request.params.not_before = now;
        request.params.not_after = now + Self::LEAF_LIFETIME;
        request.params.is_ca = IsCa::NoCa;
        request.params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];

        let leaf = request
            .signed_by(&self.issuer)
            .map_err(|e| PlayError::Signer(format!("signing the leaf: {e}")))?;

        Ok(format!("{}{}", leaf.pem(), self.root_pem()))
    }
}

impl Default for LocalSigner {
    fn default() -> Self {
        Self::new()
    }
}
