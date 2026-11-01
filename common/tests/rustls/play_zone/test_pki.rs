use std::sync::Arc;

use common::rustls::PlayZoneVerifier;
use rcgen::{
    BasicConstraints, CertificateParams, CidrSubnet, DnType, GeneralSubtree, IsCa, Issuer, KeyPair,
    KeyUsagePurpose, NameConstraints,
};
use rustls::RootCertStore;
use rustls::client::WebPkiServerVerifier;
use rustls::client::danger::{ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};

/// A throwaway root that signs leaves directly.
pub struct TestPki {
    root: CertificateDer<'static>,
    issuer: Issuer<'static, KeyPair>,
}

impl TestPki {
    /// A root name-constrained as the production play root is: permitted DNS subtree
    /// `play.bedrockvc.stream`, every IP range excluded.
    pub fn play_root(common_name: &str) -> Self {
        Self::build(common_name, true)
    }

    /// A root with no name constraints, standing in for a public root or for a play root
    /// that was issued wrongly.
    pub fn unconstrained_root(common_name: &str) -> Self {
        Self::build(common_name, false)
    }

    /// An intermediate CA signed by this one; `constrained` gives it the production
    /// name constraints. Its leaves chain to this root through it; `root()` on the result is the
    /// intermediate certificate, which a verifier receives as an intermediate.
    pub fn intermediate(&self, common_name: &str, constrained: bool) -> Self {
        let (params, key) = Self::ca_params(common_name, constrained);
        let der = params.signed_by(&key, &self.issuer).unwrap().der().clone();

        Self {
            root: der,
            issuer: Issuer::new(params, key),
        }
    }

    fn ca_params(common_name: &str, constrained: bool) -> (CertificateParams, KeyPair) {
        let mut params = CertificateParams::default();
        params
            .distinguished_name
            .push(DnType::CommonName, common_name);
        params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        params.key_usages = vec![KeyUsagePurpose::KeyCertSign];
        if constrained {
            params.name_constraints = Some(NameConstraints {
                permitted_subtrees: vec![GeneralSubtree::DnsName("play.bedrockvc.stream".into())],
                excluded_subtrees: vec![
                    GeneralSubtree::IpAddress(CidrSubnet::V4([0; 4], [0; 4])),
                    GeneralSubtree::IpAddress(CidrSubnet::V6([0; 16], [0; 16])),
                ],
            });
        }

        (params, KeyPair::generate().unwrap())
    }

    fn build(common_name: &str, constrained: bool) -> Self {
        let (params, key) = Self::ca_params(common_name, constrained);
        let root = params.self_signed(&key).unwrap().der().clone();

        Self {
            root,
            issuer: Issuer::new(params, key),
        }
    }

    pub fn root(&self) -> CertificateDer<'static> {
        self.root.clone()
    }

    /// A leaf whose SANs are exactly `names`; an entry that parses as an IP address
    /// becomes an IP SAN.
    pub fn leaf(&self, names: &[&str]) -> CertificateDer<'static> {
        let params =
            CertificateParams::new(names.iter().map(|n| n.to_string()).collect::<Vec<_>>())
                .unwrap();
        let key = KeyPair::generate().unwrap();

        params.signed_by(&key, &self.issuer).unwrap().der().clone()
    }

    /// An ordinary verifier that trusts exactly `roots`.
    pub fn fallback_trusting(roots: Vec<CertificateDer<'static>>) -> Arc<dyn ServerCertVerifier> {
        let mut store = RootCertStore::empty();
        for root in roots {
            store.add(root).unwrap();
        }
        let provider = Arc::new(rustls::crypto::aws_lc_rs::default_provider());

        WebPkiServerVerifier::builder_with_provider(Arc::new(store), provider)
            .build()
            .unwrap()
    }

    pub fn verifier(
        play_roots: Vec<CertificateDer<'static>>,
        fallback: Arc<dyn ServerCertVerifier>,
    ) -> PlayZoneVerifier {
        PlayZoneVerifier::new(play_roots, fallback).unwrap()
    }

    pub fn verify(
        verifier: &PlayZoneVerifier,
        leaf: &CertificateDer<'_>,
        name: &str,
    ) -> Result<ServerCertVerified, rustls::Error> {
        Self::verify_via(verifier, leaf, &[], name)
    }

    pub fn verify_via(
        verifier: &PlayZoneVerifier,
        leaf: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        name: &str,
    ) -> Result<ServerCertVerified, rustls::Error> {
        verifier.verify_server_cert(
            leaf,
            intermediates,
            &ServerName::try_from(name).unwrap(),
            &[],
            UnixTime::now(),
        )
    }
}
