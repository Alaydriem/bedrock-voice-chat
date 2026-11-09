use rcgen::{
    BasicConstraints, CertificateParams, CidrSubnet, DnType, GeneralSubtree, IsCa, Issuer, KeyPair,
    KeyUsagePurpose, NameConstraints,
};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};

/// A throwaway play root, name-constrained as the production one is, and a leaf it signed.
pub struct PlayChain {
    pub root: CertificateDer<'static>,
    pub leaf: CertificateDer<'static>,
    pub leaf_key: PrivateKeyDer<'static>,
}

impl PlayChain {
    pub fn new(root_name: &str, leaf_name: &str) -> Self {
        let mut params = CertificateParams::default();
        params
            .distinguished_name
            .push(DnType::CommonName, root_name);
        params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        params.key_usages = vec![KeyUsagePurpose::KeyCertSign];
        params.name_constraints = Some(NameConstraints {
            permitted_subtrees: vec![GeneralSubtree::DnsName("play.bedrockvc.stream".into())],
            excluded_subtrees: vec![
                GeneralSubtree::IpAddress(CidrSubnet::V4([0; 4], [0; 4])),
                GeneralSubtree::IpAddress(CidrSubnet::V6([0; 16], [0; 16])),
            ],
        });
        let root_key = KeyPair::generate().unwrap();
        let root = params.self_signed(&root_key).unwrap().der().clone();
        let issuer = Issuer::new(params, root_key);

        let leaf_params = CertificateParams::new(vec![leaf_name.to_string()]).unwrap();
        let leaf_key = KeyPair::generate().unwrap();
        let leaf = leaf_params.signed_by(&leaf_key, &issuer).unwrap();

        Self {
            root,
            leaf: leaf.der().clone(),
            leaf_key: PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(leaf_key.serialize_der())),
        }
    }
}
