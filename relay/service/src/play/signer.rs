use super::error::PlayError;
use super::local_signer::LocalSigner;
use super::step_ca_signer::StepCaSigner;

// Who signs play certificates. An enum, not a trait object: the set is closed.
pub enum PlaySigner {
    StepCa(StepCaSigner),
    Local(LocalSigner),
}

impl PlaySigner {
    // The chain as PEM, leaf first.
    pub async fn sign(&self, csr_pem: &str, fqdn: &str) -> Result<String, PlayError> {
        match self {
            Self::StepCa(signer) => signer.sign(csr_pem, fqdn).await,
            Self::Local(signer) => signer.sign(csr_pem, fqdn),
        }
    }
}
