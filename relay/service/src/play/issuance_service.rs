use std::sync::Arc;

use base64::Engine;
use common::curia;
use x509_parser::certification_request::X509CertificationRequest;
use x509_parser::extensions::{GeneralName, ParsedExtension};
use x509_parser::prelude::FromDer;

use super::error::PlayError;
use super::name_service::PlayNameService;
use super::signer::PlaySigner;

// Signs a certificate for the one name a node key holds.
//
// The name is the node's, never the request's: the CSR must name exactly that and
// nothing else, so a server cannot choose what it is certified for.
pub struct PlayIssuanceService {
    names: Arc<PlayNameService>,
    signer: PlaySigner,
}

impl PlayIssuanceService {
    pub fn new(names: Arc<PlayNameService>, signer: PlaySigner) -> Self {
        Self { names, signer }
    }

    pub fn new_shared(names: Arc<PlayNameService>, signer: PlaySigner) -> Arc<Self> {
        Arc::new(Self::new(names, signer))
    }

    // The chain as PEM, leaf first.
    pub async fn issue(&self, node_id: &str, csr_der: &[u8]) -> Result<String, PlayError> {
        let label = self
            .names
            .name_for(node_id)
            .await?
            .ok_or(PlayError::NoPlayName)?;
        let fqdn = self.names.fqdn(&label);

        Self::check(csr_der, &fqdn)?;
        self.names.admit_issuance(node_id).await?;

        let chain = self.signer.sign(&Self::csr_pem(csr_der), &fqdn).await?;

        curia::info!("issued a play certificate", { "name": label, "node": node_id.to_string(), "serial": Self::leaf_serial(&chain) });

        Ok(chain)
    }

    // The certificate serial of the first certificate in the chain, for the audit trail.
    fn leaf_serial(chain: &str) -> String {
        x509_parser::pem::parse_x509_pem(chain.as_bytes())
            .ok()
            .and_then(|(_, pem)| {
                pem.parse_x509()
                    .ok()
                    .map(|leaf| leaf.raw_serial_as_string())
            })
            .unwrap_or_default()
    }

    // The request's own signature must verify, its only SAN must be the node's name, and
    // a subject CN, when there is one, must be that name too: step-ca compares them.
    fn check(csr_der: &[u8], fqdn: &str) -> Result<(), PlayError> {
        let rejected = |why: &str| PlayError::CsrRejected(why.to_string());

        let (rest, csr) = X509CertificationRequest::from_der(csr_der)
            .map_err(|_| rejected("the request does not parse"))?;
        if !rest.is_empty() {
            return Err(rejected("the request has trailing data"));
        }
        csr.verify_signature()
            .map_err(|_| rejected("the request's signature does not verify"))?;

        let mut dns_names = Vec::new();
        for extension in csr.requested_extensions().into_iter().flatten() {
            if let ParsedExtension::SubjectAlternativeName(san) = extension {
                for name in &san.general_names {
                    match name {
                        GeneralName::DNSName(dns) => dns_names.push(dns.to_ascii_lowercase()),
                        _ => return Err(rejected("the request names something other than a DNS name")),
                    }
                }
            }
        }
        if dns_names != [fqdn.to_ascii_lowercase()] {
            return Err(rejected("the request must name exactly this node's play name"));
        }

        for common_name in csr.certification_request_info.subject.iter_common_name() {
            let common_name = common_name
                .as_str()
                .map_err(|_| rejected("the request's subject is unreadable"))?;
            if !common_name.eq_ignore_ascii_case(fqdn) {
                return Err(rejected("the request's subject differs from the play name"));
            }
        }

        Ok(())
    }

    fn csr_pem(csr_der: &[u8]) -> String {
        let encoded = base64::engine::general_purpose::STANDARD.encode(csr_der);
        let mut pem = String::from("-----BEGIN CERTIFICATE REQUEST-----\n");
        for line in encoded.as_bytes().chunks(64) {
            pem.push_str(std::str::from_utf8(line).expect("base64 is ASCII"));
            pem.push('\n');
        }
        pem.push_str("-----END CERTIFICATE REQUEST-----\n");
        pem
    }
}
