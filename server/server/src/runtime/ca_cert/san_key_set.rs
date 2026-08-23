//! Canonical SAN comparison for the CA certificate.
//!
//! Deciding whether `ca.crt` needs re-signing is a set comparison, not a string
//! comparison: the same SAN can be written several ways (`Example.COM` vs
//! `example.com`, an IPv6 address in either compressed or expanded form) and can
//! arrive in any order, with duplicates. Normalizing to a canonical key per entry
//! makes the comparison total and order-independent.

use std::collections::HashSet;

use anyhow::{Result, anyhow};
use rcgen::{CertificateParams, SanType};

/// A canonical, comparable set of SAN keys.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SanKeySet {
    keys: HashSet<String>,
}

impl SanKeySet {
    /// Canonical, comparable string form of a single SAN entry. DNS names are
    /// lowercased (DNS names are case-insensitive per RFC 5280 §4.2.1.6); IP
    /// addresses use their `std::net::IpAddr` Display form, which is canonical for
    /// both v4 and v6.
    pub fn normalize(san: &SanType) -> String {
        match san {
            SanType::DnsName(name) => format!("DNS:{}", name.as_ref().to_ascii_lowercase()),
            SanType::IpAddress(ip) => format!("IP:{}", ip),
            SanType::Rfc822Name(name) => format!("EMAIL:{}", name.as_ref().to_ascii_lowercase()),
            SanType::URI(uri) => format!("URI:{}", uri.as_ref()),
            other => format!("OTHER:{:?}", other),
        }
    }

    /// Build the key set from raw config strings. Routes through
    /// `rcgen::CertificateParams::new` so DNS-vs-IP detection follows rcgen's own
    /// rules, which is the same rule used when generating the cert.
    pub fn from_strings(strings: &[String]) -> Result<Self> {
        let params = CertificateParams::new(strings.to_vec())
            .map_err(|e| anyhow!("invalid SAN entry in config: {e}"))?;
        Ok(Self {
            keys: params
                .subject_alt_names
                .iter()
                .map(Self::normalize)
                .collect(),
        })
    }

    /// Read the key set out of an existing certificate PEM. Uses `x509-parser`
    /// directly because rcgen 0.14 dropped `CertificateParams::from_ca_cert_pem`;
    /// `Issuer`'s replacement does not surface SANs.
    pub fn from_certificate_pem(pem: &str) -> Result<Self> {
        use x509_parser::extensions::{GeneralName, ParsedExtension};
        use x509_parser::prelude::*;

        let (_, parsed_pem) = x509_parser::pem::parse_x509_pem(pem.as_bytes())
            .map_err(|e| anyhow!("failed to parse existing ca.crt PEM: {e}"))?;
        let (_, cert) = X509Certificate::from_der(&parsed_pem.contents)
            .map_err(|e| anyhow!("failed to parse existing ca.crt DER: {e}"))?;

        let mut keys = HashSet::new();
        for ext in cert.extensions() {
            if let ParsedExtension::SubjectAlternativeName(san) = ext.parsed_extension() {
                for gn in &san.general_names {
                    match gn {
                        GeneralName::DNSName(name) => {
                            keys.insert(format!("DNS:{}", name.to_ascii_lowercase()));
                        }
                        GeneralName::IPAddress(bytes) => {
                            if bytes.len() == 4 {
                                let arr: [u8; 4] = (*bytes).try_into().unwrap();
                                keys.insert(format!("IP:{}", std::net::Ipv4Addr::from(arr)));
                            } else if bytes.len() == 16 {
                                let arr: [u8; 16] = (*bytes).try_into().unwrap();
                                keys.insert(format!("IP:{}", std::net::Ipv6Addr::from(arr)));
                            } else {
                                keys.insert(format!("IP:invalid({} bytes)", bytes.len()));
                            }
                        }
                        GeneralName::RFC822Name(name) => {
                            keys.insert(format!("EMAIL:{}", name.to_ascii_lowercase()));
                        }
                        GeneralName::URI(uri) => {
                            keys.insert(format!("URI:{}", uri));
                        }
                        other => {
                            keys.insert(format!("OTHER:{:?}", other));
                        }
                    }
                }
            }
        }
        Ok(Self { keys })
    }

    /// Whether a canonical key is present, e.g. `"DNS:localhost"` or
    /// `"IP:127.0.0.1"`.
    pub fn contains(&self, key: &str) -> bool {
        self.keys.contains(key)
    }

    /// Deterministic ordering, for log output and for assertions that need a
    /// stable sequence out of an unordered set.
    pub fn sorted(&self) -> Vec<String> {
        let mut v: Vec<String> = self.keys.iter().cloned().collect();
        v.sort();
        v
    }

    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    pub fn len(&self) -> usize {
        self.keys.len()
    }
}
