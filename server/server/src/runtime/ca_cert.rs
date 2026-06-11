//! CA certificate management for the QUIC server.
//!
//! The CA cert serves two roles:
//! 1. The QUIC server's TLS leaf cert, presented during the handshake.
//! 2. The trust root that issues player mTLS client certs.
//!
//! The keypair (`ca.key`) is generated once on first start and never replaced.
//! The cert (`ca.crt`) is re-signed with the same keypair whenever the configured
//! SAN set drifts from what is embedded in the cert. This preserves the chain of
//! trust for every player cert ever issued, and keeps the Subject DN + SPKI of
//! the trust anchor stable for clients that have pinned the root cert.

use anyhow::{anyhow, Context, Result};
use rcgen::{
    CertificateParams, DistinguishedName, ExtendedKeyUsagePurpose, IsCa, KeyPair,
    KeyUsagePurpose, SanType,
};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use time::{Duration, OffsetDateTime};
use tracing::info;

const CA_SUBJECT_CN: &str = "Bedrock Voice Chat";

/// Manages generation and persistence of the CA certificate and keypair.
pub struct CaCertManager {
    certs_path: String,
}

impl CaCertManager {
    pub fn new(certs_path: &str) -> Self {
        Self {
            certs_path: certs_path.to_string(),
        }
    }

    /// Ensure `ca.crt` and `ca.key` exist at the configured path and that the
    /// cert's SANs match `san_strings`. Returns `(cert_pem, key_pem)` on success.
    ///
    /// Behavior:
    /// - If `ca.key` is missing, a fresh keypair is generated and persisted.
    /// - If `ca.key` exists, it is reused and never rewritten.
    /// - If `ca.crt` is missing or its SAN set differs from `san_strings`, the cert
    ///   is re-signed with the (existing or freshly generated) keypair and written
    ///   atomically.
    pub fn ensure(&self, san_strings: &[String]) -> Result<(String, String)> {
        let dir = Path::new(&self.certs_path);
        if !dir.exists() {
            fs::create_dir_all(dir)
                .with_context(|| format!("creating certs dir {}", dir.display()))?;
        }
        let cert_path = dir.join("ca.crt");
        let key_path = dir.join("ca.key");

        let (keypair, key_pem) = Self::load_or_create_keypair(&key_path)?;
        let desired = Self::build_desired_san_keys(san_strings)?;

        if cert_path.exists() {
            let existing_pem = fs::read_to_string(&cert_path)
                .with_context(|| format!("reading ca.crt at {}", cert_path.display()))?;
            let existing = Self::parse_existing_san_keys(&existing_pem)?;
            if existing == desired {
                return Ok((existing_pem, key_pem));
            }
            info!(
                "ca.crt SAN set drifted from config (was: {:?}, now: {:?}); re-signing with existing keypair",
                Self::sorted(&existing),
                Self::sorted(&desired),
            );
        }

        let new_cert_pem = Self::sign_ca_cert(&keypair, san_strings)?;
        Self::write_atomically(&cert_path, &new_cert_pem)?;
        Ok((new_cert_pem, key_pem))
    }

    /// Canonical, comparable string form of a SAN entry. DNS names are lowercased
    /// (DNS names are case-insensitive per RFC 5280 §4.2.1.6); IP addresses use
    /// their `std::net::IpAddr` Display form, which is canonical for both v4 and v6.
    fn san_key(san: &SanType) -> String {
        match san {
            SanType::DnsName(name) => format!("DNS:{}", name.as_ref().to_ascii_lowercase()),
            SanType::IpAddress(ip) => format!("IP:{}", ip),
            SanType::Rfc822Name(name) => format!("EMAIL:{}", name.as_ref().to_ascii_lowercase()),
            SanType::URI(uri) => format!("URI:{}", uri.as_ref()),
            other => format!("OTHER:{:?}", other),
        }
    }

    /// Build the desired SAN key set from raw config strings. Routes through
    /// `rcgen::CertificateParams::new` so DNS-vs-IP detection follows rcgen's own
    /// rules, which is the same rule used when generating the cert.
    fn build_desired_san_keys(strings: &[String]) -> Result<HashSet<String>> {
        let params = CertificateParams::new(strings.to_vec())
            .map_err(|e| anyhow!("invalid SAN entry in config: {e}"))?;
        Ok(params.subject_alt_names.iter().map(Self::san_key).collect())
    }

    /// Read the SAN set out of an existing CA cert PEM. Uses `x509-parser` directly
    /// because rcgen 0.14 dropped `CertificateParams::from_ca_cert_pem`; `Issuer`'s
    /// replacement does not surface SANs.
    fn parse_existing_san_keys(pem: &str) -> Result<HashSet<String>> {
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
        Ok(keys)
    }

    /// Load `ca.key` from disk if it exists, otherwise generate a new keypair and
    /// write it. Returns the keypair and the PEM we have on disk for it.
    ///
    /// This is the trust-anchor invariant: the keypair is generated **exactly once**
    /// per `certs_path` for the lifetime of the deployment. Replacing it would
    /// invalidate every leaf cert ever signed by it.
    fn load_or_create_keypair(key_path: &Path) -> Result<(KeyPair, String)> {
        if key_path.exists() {
            let pem = fs::read_to_string(key_path)
                .with_context(|| format!("reading ca.key at {}", key_path.display()))?;
            let kp = KeyPair::from_pem(&pem)
                .map_err(|e| anyhow!("parsing ca.key at {}: {e}", key_path.display()))?;
            Ok((kp, pem))
        } else {
            let kp = KeyPair::generate()
                .map_err(|e| anyhow!("generating ca.key: {e}"))?;
            let pem = kp.serialize_pem();
            fs::write(key_path, &pem)
                .with_context(|| format!("writing ca.key at {}", key_path.display()))?;
            Ok((kp, pem))
        }
    }

    /// Self-sign a CA cert with `keypair`, embedding `san_strings` and the fixed
    /// Subject DN, IsCa, key usage, and extended key usage that this server has
    /// always used. Validity starts 3 days in the past (matches prior behavior to
    /// tolerate clock skew on first connect) and runs for 90 days; a periodic
    /// re-sign on operator restart keeps the cert fresh without changing the trust
    /// anchor identity.
    fn sign_ca_cert(keypair: &KeyPair, san_strings: &[String]) -> Result<String> {
        let mut params = CertificateParams::new(san_strings.to_vec())
            .map_err(|e| anyhow!("invalid SAN entry: {e}"))?;
        let mut dn = DistinguishedName::new();
        dn.push(rcgen::DnType::CommonName, CA_SUBJECT_CN);
        params.distinguished_name = dn;
        params.is_ca = IsCa::NoCa;
        params.use_authority_key_identifier_extension = true;
        params.key_usages = vec![KeyUsagePurpose::KeyCertSign];
        params.extended_key_usages = vec![
            ExtendedKeyUsagePurpose::ClientAuth,
            ExtendedKeyUsagePurpose::ServerAuth,
        ];
        params.not_before = OffsetDateTime::now_utc()
            .checked_sub(Duration::days(3))
            .ok_or_else(|| anyhow!("not_before underflow"))?;
        params.not_after = OffsetDateTime::now_utc()
            .checked_add(Duration::days(90))
            .ok_or_else(|| anyhow!("not_after overflow"))?;
        let cert = params
            .self_signed(keypair)
            .map_err(|e| anyhow!("self-signing ca.crt: {e}"))?;
        Ok(cert.pem())
    }

    /// Write `content` to `path` atomically: write to `<path>.tmp`, then rename.
    /// Prevents leaving a half-written `ca.crt` if the process is killed mid-write.
    fn write_atomically(path: &Path, content: &str) -> Result<()> {
        let tmp: PathBuf = {
            let mut p = path.as_os_str().to_owned();
            p.push(".tmp");
            PathBuf::from(p)
        };
        fs::write(&tmp, content)
            .with_context(|| format!("writing tmp file {}", tmp.display()))?;
        fs::rename(&tmp, path)
            .with_context(|| format!("renaming {} -> {}", tmp.display(), path.display()))?;
        Ok(())
    }

    fn sorted(set: &HashSet<String>) -> Vec<String> {
        let mut v: Vec<_> = set.iter().cloned().collect();
        v.sort();
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs as stdfs;
    use tempfile::TempDir;

    fn s(v: &str) -> String { v.to_string() }

    #[test]
    fn san_key_normalizes_dns() {
        let san = SanType::DnsName("Example.com".to_string().try_into().unwrap());
        assert_eq!(CaCertManager::san_key(&san), "DNS:example.com");
    }

    #[test]
    fn san_key_normalizes_ipv4() {
        let san = SanType::IpAddress(std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)));
        assert_eq!(CaCertManager::san_key(&san), "IP:127.0.0.1");
    }

    #[test]
    fn san_key_normalizes_ipv6() {
        let san = SanType::IpAddress(std::net::IpAddr::V6(std::net::Ipv6Addr::LOCALHOST));
        assert_eq!(CaCertManager::san_key(&san), "IP:::1");
    }

    #[test]
    fn build_desired_san_keys_mixes_dns_and_ip() {
        let got = CaCertManager::build_desired_san_keys(&[s("localhost"), s("127.0.0.1"), s("::1")]).unwrap();
        let want: HashSet<String> = ["DNS:localhost", "IP:127.0.0.1", "IP:::1"]
            .iter().map(|s| s.to_string()).collect();
        assert_eq!(got, want);
    }

    #[test]
    fn build_desired_san_keys_dedupes_and_is_order_independent() {
        let a = CaCertManager::build_desired_san_keys(&[s("a.example"), s("b.example"), s("a.example")]).unwrap();
        let b = CaCertManager::build_desired_san_keys(&[s("b.example"), s("a.example")]).unwrap();
        assert_eq!(a, b);
        assert_eq!(a.len(), 2);
    }

    #[test]
    fn build_desired_san_keys_lowercases_dns() {
        let got = CaCertManager::build_desired_san_keys(&[s("Mixed.Case.EXAMPLE")]).unwrap();
        assert!(got.contains("DNS:mixed.case.example"));
    }

    fn build_test_cert_pem(san_strings: &[String]) -> String {
        let params = CertificateParams::new(san_strings.to_vec()).unwrap();
        let kp = KeyPair::generate().unwrap();
        params.self_signed(&kp).unwrap().pem()
    }

    #[test]
    fn parse_existing_san_keys_reads_dns_and_ip() {
        let pem = build_test_cert_pem(&[s("foo.example"), s("10.0.0.1"), s("::1")]);
        let got = CaCertManager::parse_existing_san_keys(&pem).unwrap();
        let want: HashSet<String> = ["DNS:foo.example", "IP:10.0.0.1", "IP:::1"]
            .iter().map(|s| s.to_string()).collect();
        assert_eq!(got, want);
    }

    #[test]
    fn parse_existing_san_keys_returns_empty_when_no_san_extension() {
        let mut params = CertificateParams::default();
        params.distinguished_name = {
            let mut dn = DistinguishedName::new();
            dn.push(rcgen::DnType::CommonName, "no-san-here");
            dn
        };
        let kp = KeyPair::generate().unwrap();
        let pem = params.self_signed(&kp).unwrap().pem();
        let got = CaCertManager::parse_existing_san_keys(&pem).unwrap();
        assert!(got.is_empty());
    }

    #[test]
    fn parse_existing_san_keys_errors_on_garbage() {
        let err = CaCertManager::parse_existing_san_keys("not a pem").unwrap_err();
        assert!(format!("{err}").contains("parse"));
    }

    #[test]
    fn load_or_create_keypair_creates_when_absent() {
        let dir = TempDir::new().unwrap();
        let key_path = dir.path().join("ca.key");
        let (kp, pem) = CaCertManager::load_or_create_keypair(&key_path).unwrap();
        assert!(key_path.exists());
        assert_eq!(stdfs::read_to_string(&key_path).unwrap(), pem);
        let kp_pem = kp.serialize_pem();
        assert_eq!(kp_pem, pem);
    }

    #[test]
    fn load_or_create_keypair_reuses_existing_file() {
        let dir = TempDir::new().unwrap();
        let key_path = dir.path().join("ca.key");
        let (_kp1, pem1) = CaCertManager::load_or_create_keypair(&key_path).unwrap();
        let mtime1 = stdfs::metadata(&key_path).unwrap().modified().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
        let (_kp2, pem2) = CaCertManager::load_or_create_keypair(&key_path).unwrap();
        let mtime2 = stdfs::metadata(&key_path).unwrap().modified().unwrap();
        assert_eq!(pem1, pem2, "key PEM must not change on reload");
        assert_eq!(mtime1, mtime2, "ca.key file must not be rewritten on reload");
    }

    #[test]
    fn load_or_create_keypair_errors_when_dir_missing() {
        let bogus = std::path::PathBuf::from("/nonexistent/definitely/missing/ca.key");
        let err = CaCertManager::load_or_create_keypair(&bogus).unwrap_err();
        let s = format!("{err}");
        assert!(s.to_lowercase().contains("ca.key") || s.to_lowercase().contains("no such"));
    }

    #[test]
    fn sign_ca_cert_embeds_configured_sans() {
        let kp = KeyPair::generate().unwrap();
        let pem = CaCertManager::sign_ca_cert(&kp, &[s("a.example"), s("10.0.0.1")]).unwrap();
        let sans = CaCertManager::parse_existing_san_keys(&pem).unwrap();
        assert!(sans.contains("DNS:a.example"));
        assert!(sans.contains("IP:10.0.0.1"));
        assert_eq!(sans.len(), 2);
    }

    #[test]
    fn sign_ca_cert_uses_fixed_subject_cn() {
        use x509_parser::prelude::*;
        let kp = KeyPair::generate().unwrap();
        let pem = CaCertManager::sign_ca_cert(&kp, &[s("localhost")]).unwrap();
        let (_, parsed_pem) = x509_parser::pem::parse_x509_pem(pem.as_bytes()).unwrap();
        let (_, cert) = X509Certificate::from_der(&parsed_pem.contents).unwrap();
        let cn_attr = cert
            .tbs_certificate
            .subject
            .iter_common_name()
            .next()
            .expect("CN must be set");
        let cn = cn_attr.as_str().expect("CN must be a string");
        assert_eq!(cn, CA_SUBJECT_CN);
    }

    #[test]
    fn sign_ca_cert_includes_eku_client_and_server_auth() {
        use x509_parser::extensions::ParsedExtension;
        use x509_parser::prelude::*;
        let kp = KeyPair::generate().unwrap();
        let pem = CaCertManager::sign_ca_cert(&kp, &[s("localhost")]).unwrap();
        let (_, parsed_pem) = x509_parser::pem::parse_x509_pem(pem.as_bytes()).unwrap();
        let (_, cert) = X509Certificate::from_der(&parsed_pem.contents).unwrap();
        let eku = cert
            .extensions()
            .iter()
            .find_map(|ext| {
                if let ParsedExtension::ExtendedKeyUsage(eku) = ext.parsed_extension() {
                    Some(eku)
                } else {
                    None
                }
            })
            .expect("EKU extension must be present");
        assert!(eku.client_auth);
        assert!(eku.server_auth);
    }

    #[test]
    fn sign_ca_cert_includes_keycertsign_key_usage() {
        use x509_parser::extensions::ParsedExtension;
        use x509_parser::prelude::*;
        let kp = KeyPair::generate().unwrap();
        let pem = CaCertManager::sign_ca_cert(&kp, &[s("localhost")]).unwrap();
        let (_, parsed_pem) = x509_parser::pem::parse_x509_pem(pem.as_bytes()).unwrap();
        let (_, cert) = X509Certificate::from_der(&parsed_pem.contents).unwrap();
        let ku = cert
            .extensions()
            .iter()
            .find_map(|ext| {
                if let ParsedExtension::KeyUsage(ku) = ext.parsed_extension() {
                    Some(ku)
                } else {
                    None
                }
            })
            .expect("KeyUsage extension must be present");
        assert!(ku.key_cert_sign());
    }

    #[test]
    fn sign_ca_cert_with_same_keypair_produces_byte_identical_spki() {
        use x509_parser::prelude::*;

        let kp = KeyPair::generate().unwrap();
        let pem_a = CaCertManager::sign_ca_cert(&kp, &[s("a.example")]).unwrap();
        let pem_b = CaCertManager::sign_ca_cert(&kp, &[s("b.example"), s("c.example")]).unwrap();

        let spki_of = |pem: &str| -> Vec<u8> {
            let (_, p) = x509_parser::pem::parse_x509_pem(pem.as_bytes()).unwrap();
            let (_, cert) = X509Certificate::from_der(&p.contents).unwrap();
            cert.tbs_certificate.subject_pki.raw.to_vec()
        };
        assert_eq!(spki_of(&pem_a), spki_of(&pem_b));
    }

    #[test]
    fn write_atomically_creates_file() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("ca.crt");
        CaCertManager::write_atomically(&target, "hello").unwrap();
        assert_eq!(stdfs::read_to_string(&target).unwrap(), "hello");
        let leftover: Vec<_> = stdfs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().contains(".tmp"))
            .collect();
        assert!(leftover.is_empty(), "no .tmp files should remain");
    }

    #[test]
    fn write_atomically_overwrites_existing() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("ca.crt");
        stdfs::write(&target, "old").unwrap();
        CaCertManager::write_atomically(&target, "new").unwrap();
        assert_eq!(stdfs::read_to_string(&target).unwrap(), "new");
    }

    fn read_pems(certs_path: &Path) -> (String, String) {
        let cert = stdfs::read_to_string(certs_path.join("ca.crt")).unwrap();
        let key = stdfs::read_to_string(certs_path.join("ca.key")).unwrap();
        (cert, key)
    }

    #[test]
    fn ensure_first_run_generates_both_files() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().to_str().unwrap();
        let mgr = CaCertManager::new(path);
        let (cert_pem, key_pem) = mgr.ensure(&[s("localhost"), s("127.0.0.1")]).unwrap();
        assert!(dir.path().join("ca.crt").exists());
        assert!(dir.path().join("ca.key").exists());
        let (disk_cert, disk_key) = read_pems(dir.path());
        assert_eq!(cert_pem, disk_cert);
        assert_eq!(key_pem, disk_key);
        let sans = CaCertManager::parse_existing_san_keys(&cert_pem).unwrap();
        assert!(sans.contains("DNS:localhost"));
        assert!(sans.contains("IP:127.0.0.1"));
    }

    #[test]
    fn ensure_creates_certs_dir_if_missing() {
        let outer = TempDir::new().unwrap();
        let nested = outer.path().join("does/not/exist/yet");
        let path = nested.to_str().unwrap();
        let mgr = CaCertManager::new(path);
        mgr.ensure(&[s("localhost")]).unwrap();
        assert!(nested.join("ca.crt").exists());
        assert!(nested.join("ca.key").exists());
    }

    #[test]
    fn ensure_is_noop_when_sans_match() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().to_str().unwrap();
        let sans = vec![s("localhost"), s("127.0.0.1")];
        let mgr = CaCertManager::new(path);
        let (cert1, key1) = mgr.ensure(&sans).unwrap();
        let cert_mtime_1 = stdfs::metadata(dir.path().join("ca.crt")).unwrap().modified().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
        let (cert2, key2) = mgr.ensure(&sans).unwrap();
        let cert_mtime_2 = stdfs::metadata(dir.path().join("ca.crt")).unwrap().modified().unwrap();
        assert_eq!(cert1, cert2, "cert PEM must be byte-equal across no-op runs");
        assert_eq!(key1, key2, "key PEM must be byte-equal across no-op runs");
        assert_eq!(cert_mtime_1, cert_mtime_2, "ca.crt must not be rewritten");
    }

    #[test]
    fn ensure_is_order_and_dedup_insensitive() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().to_str().unwrap();
        let mgr = CaCertManager::new(path);
        mgr.ensure(&[s("a.example"), s("b.example")]).unwrap();
        let cert_mtime_1 = stdfs::metadata(dir.path().join("ca.crt")).unwrap().modified().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
        mgr.ensure(&[s("b.example"), s("a.example"), s("a.example")]).unwrap();
        let cert_mtime_2 = stdfs::metadata(dir.path().join("ca.crt")).unwrap().modified().unwrap();
        assert_eq!(cert_mtime_1, cert_mtime_2, "ca.crt must not be rewritten on equivalent SAN set");
    }

    #[test]
    fn ensure_rewrites_cert_when_sans_drift_but_keeps_key() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().to_str().unwrap();
        let mgr = CaCertManager::new(path);
        let (cert1, key1) = mgr.ensure(&[s("a.example")]).unwrap();
        let key_mtime_1 = stdfs::metadata(dir.path().join("ca.key")).unwrap().modified().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
        let (cert2, key2) = mgr.ensure(&[s("a.example"), s("b.example")]).unwrap();
        let key_mtime_2 = stdfs::metadata(dir.path().join("ca.key")).unwrap().modified().unwrap();

        assert_ne!(cert1, cert2, "cert PEM must change when SANs drift");
        assert_eq!(key1, key2, "key PEM must not change when SANs drift");
        assert_eq!(key_mtime_1, key_mtime_2, "ca.key file must not be rewritten");

        let sans2 = CaCertManager::parse_existing_san_keys(&cert2).unwrap();
        assert!(sans2.contains("DNS:a.example"));
        assert!(sans2.contains("DNS:b.example"));
        assert_eq!(sans2.len(), 2);
    }

    #[test]
    fn ensure_regenerates_cert_when_only_key_present() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().to_str().unwrap();
        let kp = KeyPair::generate().unwrap();
        stdfs::write(dir.path().join("ca.key"), kp.serialize_pem()).unwrap();

        let mgr = CaCertManager::new(path);
        let (cert, key) = mgr.ensure(&[s("localhost")]).unwrap();
        assert!(dir.path().join("ca.crt").exists());
        assert_eq!(key, kp.serialize_pem());
        let sans = CaCertManager::parse_existing_san_keys(&cert).unwrap();
        assert!(sans.contains("DNS:localhost"));
    }

    #[test]
    fn ensure_errors_on_corrupt_existing_cert() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().to_str().unwrap();
        let kp = KeyPair::generate().unwrap();
        stdfs::write(dir.path().join("ca.key"), kp.serialize_pem()).unwrap();
        stdfs::write(dir.path().join("ca.crt"), "not a real PEM").unwrap();
        let mgr = CaCertManager::new(path);
        let err = mgr.ensure(&[s("localhost")]).unwrap_err();
        assert!(format!("{err}").contains("ca.crt"));
    }

    fn der_of(pem_str: &str) -> Vec<u8> {
        let (_, p) = x509_parser::pem::parse_x509_pem(pem_str.as_bytes()).unwrap();
        p.contents.to_vec()
    }

    #[test]
    fn drift_resign_preserves_subject_dn_bytes() {
        use x509_parser::prelude::*;

        let dir = TempDir::new().unwrap();
        let path = dir.path().to_str().unwrap();
        let mgr = CaCertManager::new(path);
        let (cert_a, _) = mgr.ensure(&[s("a.example")]).unwrap();
        let (cert_b, _) = mgr.ensure(&[s("a.example"), s("b.example")]).unwrap();

        let der_a = der_of(&cert_a);
        let der_b = der_of(&cert_b);
        let (_, ca) = X509Certificate::from_der(&der_a).unwrap();
        let (_, cb) = X509Certificate::from_der(&der_b).unwrap();
        assert_eq!(
            ca.tbs_certificate.subject.as_raw(),
            cb.tbs_certificate.subject.as_raw(),
            "Subject DN bytes must be identical across re-sign"
        );
    }

    #[test]
    fn drift_resign_preserves_spki_bytes() {
        use x509_parser::prelude::*;

        let dir = TempDir::new().unwrap();
        let path = dir.path().to_str().unwrap();
        let mgr = CaCertManager::new(path);
        let (cert_a, _) = mgr.ensure(&[s("a.example")]).unwrap();
        let (cert_b, _) = mgr.ensure(&[s("a.example"), s("b.example")]).unwrap();

        let der_a = der_of(&cert_a);
        let der_b = der_of(&cert_b);
        let (_, ca) = X509Certificate::from_der(&der_a).unwrap();
        let (_, cb) = X509Certificate::from_der(&der_b).unwrap();
        assert_eq!(
            ca.tbs_certificate.subject_pki.raw,
            cb.tbs_certificate.subject_pki.raw,
            "SubjectPublicKeyInfo bytes must be identical across re-sign"
        );
    }

    #[test]
    fn drift_resign_preserves_subject_key_identifier() {
        use x509_parser::prelude::*;

        let dir = TempDir::new().unwrap();
        let path = dir.path().to_str().unwrap();
        let mgr = CaCertManager::new(path);
        let (cert_a, _) = mgr.ensure(&[s("a.example")]).unwrap();
        let (cert_b, _) = mgr.ensure(&[s("a.example"), s("b.example")]).unwrap();

        let der_a = der_of(&cert_a);
        let der_b = der_of(&cert_b);
        let (_, ca) = X509Certificate::from_der(&der_a).unwrap();
        let (_, cb) = X509Certificate::from_der(&der_b).unwrap();

        let ski = |c: &X509Certificate| -> Option<Vec<u8>> {
            for ext in c.extensions() {
                if let ParsedExtension::SubjectKeyIdentifier(s) = ext.parsed_extension() {
                    return Some(s.0.to_vec());
                }
            }
            None
        };
        match (ski(&ca), ski(&cb)) {
            (Some(a), Some(b)) => assert_eq!(a, b, "SKI must be identical across re-sign"),
            (None, None) => {}
            (a, b) => panic!("SKI presence differs across re-sign: {a:?} vs {b:?}"),
        }
    }

    /// Mints a player-style leaf cert signed by the given root. Mirrors what
    /// `CertificateService::sign_player_cert` does in the real codebase.
    fn mint_leaf_signed_by_root(root_pem: &str, root_key_pem: &str, leaf_cn: &str) -> String {
        use rcgen::Issuer;

        let root_kp = KeyPair::from_pem(root_key_pem).unwrap();
        let issuer = Issuer::from_ca_cert_pem(root_pem, root_kp).unwrap();

        let mut leaf_params = CertificateParams::default();
        leaf_params.distinguished_name = {
            let mut dn = DistinguishedName::new();
            dn.push(rcgen::DnType::CommonName, leaf_cn);
            dn
        };
        leaf_params.extended_key_usages = vec![
            ExtendedKeyUsagePurpose::ClientAuth,
            ExtendedKeyUsagePurpose::ServerAuth,
        ];
        leaf_params.not_before = OffsetDateTime::now_utc()
            .checked_sub(Duration::days(1))
            .unwrap();
        leaf_params.not_after = OffsetDateTime::now_utc()
            .checked_add(Duration::days(30))
            .unwrap();

        let leaf_kp = KeyPair::generate().unwrap();
        let leaf_cert = leaf_params.signed_by(&leaf_kp, &issuer).unwrap();
        leaf_cert.pem()
    }

    #[test]
    fn leaf_signed_before_drift_still_verifies_against_post_drift_root() {
        use ring::signature;
        use x509_parser::prelude::*;

        let dir = TempDir::new().unwrap();
        let path = dir.path().to_str().unwrap();
        let mgr = CaCertManager::new(path);

        let (root_pem_pre, key_pem) = mgr.ensure(&[s("a.example")]).unwrap();
        let leaf_pem = mint_leaf_signed_by_root(&root_pem_pre, &key_pem, "player:alice");

        let (root_pem_post, key_pem_2) =
            mgr.ensure(&[s("a.example"), s("b.example")]).unwrap();
        assert_eq!(key_pem, key_pem_2, "key PEM must be unchanged");
        assert_ne!(root_pem_pre, root_pem_post, "root cert must have re-signed");

        let leaf_der = der_of(&leaf_pem);
        let root_der_post = der_of(&root_pem_post);
        let (_, leaf_x509) = X509Certificate::from_der(&leaf_der).unwrap();
        let (_, root_x509_post) = X509Certificate::from_der(&root_der_post).unwrap();

        let pubkey_bytes = root_x509_post
            .tbs_certificate
            .subject_pki
            .subject_public_key
            .data
            .to_vec();
        let tbs = leaf_x509.tbs_certificate.as_ref();
        let sig = leaf_x509.signature_value.data.as_ref();

        signature::UnparsedPublicKey::new(&signature::ECDSA_P256_SHA256_ASN1, &pubkey_bytes)
            .verify(tbs, sig)
            .expect("leaf signed by pre-drift root must verify against post-drift root pubkey");

        let root_tbs = root_x509_post.tbs_certificate.as_ref();
        let root_sig = root_x509_post.signature_value.data.as_ref();
        signature::UnparsedPublicKey::new(&signature::ECDSA_P256_SHA256_ASN1, &pubkey_bytes)
            .verify(root_tbs, root_sig)
            .expect("post-drift self-sig must verify");

        assert_eq!(
            leaf_x509.tbs_certificate.issuer.as_raw(),
            root_x509_post.tbs_certificate.subject.as_raw(),
            "leaf issuer DN must still match new root subject DN"
        );
    }

    #[test]
    fn leaf_minted_after_drift_also_verifies_against_post_drift_root() {
        use ring::signature;
        use x509_parser::prelude::*;

        let dir = TempDir::new().unwrap();
        let path = dir.path().to_str().unwrap();
        let mgr = CaCertManager::new(path);

        let (_root_pre, _) = mgr.ensure(&[s("a.example")]).unwrap();
        let (root_post, key_post) =
            mgr.ensure(&[s("a.example"), s("b.example")]).unwrap();

        let leaf_pem = mint_leaf_signed_by_root(&root_post, &key_post, "player:bob");

        let leaf_der = der_of(&leaf_pem);
        let root_der = der_of(&root_post);
        let (_, leaf_x509) = X509Certificate::from_der(&leaf_der).unwrap();
        let (_, root_x509) = X509Certificate::from_der(&root_der).unwrap();
        let pubkey_bytes = root_x509
            .tbs_certificate
            .subject_pki
            .subject_public_key
            .data
            .to_vec();
        let tbs = leaf_x509.tbs_certificate.as_ref();
        let sig = leaf_x509.signature_value.data.as_ref();
        signature::UnparsedPublicKey::new(&signature::ECDSA_P256_SHA256_ASN1, &pubkey_bytes)
            .verify(tbs, sig)
            .expect("post-drift-minted leaf must verify against post-drift root pubkey");
    }
}
