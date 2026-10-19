mod pending_join;
mod roster_member;
mod safety_code;
mod staged_add;
mod storage_wiper;

pub use pending_join::PendingJoin;
pub use roster_member::RosterMember;
pub use safety_code::SafetyCode;
pub use staged_add::StagedAdd;

use storage_wiper::StorageWiper;

use anyhow::anyhow;
use openmls::prelude::{
    BasicCredential, Ciphersuite, CredentialWithKey, KeyPackage, KeyPackageIn, LeafNodeIndex,
    LeafNodeParameters, MlsGroup, MlsGroupCreateConfig, MlsMessageIn, ProcessedMessageContent,
    ProtocolVersion, Sender,
    tls_codec::{DeserializeBytes, Serialize},
};
use openmls_basic_credential::SignatureKeyPair;
use openmls_libcrux_crypto::Provider;
use hkdf::Hkdf;
use openmls_traits::OpenMlsProvider;
use sha2::Sha256;
use zeroize::Zeroizing;

/// One member's view of a protected group: OpenMLS with the X-Wing suite, kept in memory only.
///
/// Nothing here is persisted. OpenMLS's storage is the provider's in-memory store, so the group's
/// secrets live exactly as long as this value and a restart means rejoining.
pub struct MlsGroupState {
    provider: Provider,
    signer: SignatureKeyPair,
    group: MlsGroup,
}

impl MlsGroupState {
    pub const SUITE: Ciphersuite = Ciphersuite::MLS_256_XWING_CHACHA20POLY1305_SHA256_Ed25519;
    const SFRAME_LABEL: &'static str = "SFrame 1.0 Base Key";
    const SENDER_LABEL: &'static [u8] = b"bvc SFrame sender base key";
    const SFRAME_KEY_LEN: usize = 32;

    pub fn create(identity: &common::PlayerIdentity) -> anyhow::Result<Self> {
        let (provider, signer, credential) = Self::identity_material(identity)?;
        let config = MlsGroupCreateConfig::builder()
            .ciphersuite(Self::SUITE)
            .use_ratchet_tree_extension(true)
            .build();
        let group = MlsGroup::new(&provider, &signer, &config, credential)?;
        Ok(Self::from_parts(provider, signer, group))
    }

    /// A fresh KeyPackage to offer an admitter, and the pending state that can join with the
    /// Welcome it produces.
    pub fn key_package(identity: &common::PlayerIdentity) -> anyhow::Result<(PendingJoin, Vec<u8>)> {
        let (provider, signer, credential) = Self::identity_material(identity)?;
        let bundle = KeyPackage::builder().build(Self::SUITE, &provider, &signer, credential)?;
        let bytes = bundle.key_package().tls_serialize_detached()?;
        Ok((PendingJoin::new(provider, signer), bytes))
    }

    /// The identity a KeyPackage claims, read without validating it: only for rate-limiting a
    /// joiner before deciding whether to spend any work on it.
    pub fn key_package_identity(key_package: &[u8]) -> Option<common::PlayerIdentity> {
        let package = KeyPackageIn::tls_deserialize_exact_bytes(key_package).ok()?;
        let credential = BasicCredential::try_from(package.unverified_credential().credential).ok()?;
        std::str::from_utf8(credential.identity()).ok()?.parse().ok()
    }

    pub(crate) fn from_parts(provider: Provider, signer: SignatureKeyPair, group: MlsGroup) -> Self {
        Self {
            provider,
            signer,
            group,
        }
    }

    pub fn add(&mut self, key_package: &[u8]) -> anyhow::Result<StagedAdd> {
        let package = KeyPackageIn::tls_deserialize_exact_bytes(key_package)?
            .validate(self.provider.crypto(), ProtocolVersion::Mls10)?;
        let (commit, welcome, _) =
            self.group
                .add_members(&self.provider, &self.signer, &[package])?;
        Ok(StagedAdd {
            commit: commit.tls_serialize_detached()?,
            welcome: welcome.tls_serialize_detached()?,
        })
    }

    pub fn remove(&mut self, leaf: u32) -> anyhow::Result<Vec<u8>> {
        let (commit, _, _) = self.group.remove_members(
            &self.provider,
            &self.signer,
            &[LeafNodeIndex::new(leaf)],
        )?;
        Ok(commit.tls_serialize_detached()?)
    }

    pub fn self_update(&mut self) -> anyhow::Result<Vec<u8>> {
        let bundle =
            self.group
                .self_update(&self.provider, &self.signer, LeafNodeParameters::default())?;
        Ok(bundle.into_commit().tls_serialize_detached()?)
    }

    pub fn merge_pending(&mut self) -> anyhow::Result<()> {
        Ok(self.group.merge_pending_commit(&self.provider)?)
    }

    pub fn discard_pending(&mut self) {
        let _ = self.group.clear_pending_commit(self.provider.storage());
    }

    /// Applies another member's commit.
    pub fn process(&mut self, message: &[u8]) -> anyhow::Result<()> {
        let processed = self.process_message(message)?;
        match processed.into_content() {
            ProcessedMessageContent::StagedCommitMessage(staged) => {
                self.group.merge_staged_commit(&self.provider, *staged)?;
                Ok(())
            }
            _ => Err(anyhow!("expected a commit")),
        }
    }

    /// Seals a short application message only current members can open, authenticated as this
    /// member's leaf.
    pub fn seal_application(&mut self, data: &[u8]) -> anyhow::Result<Vec<u8>> {
        let message = self
            .group
            .create_message(&self.provider, &self.signer, data)?;
        Ok(message.tls_serialize_detached()?)
    }

    /// Opens an application message, returning the leaf that sent it.
    pub fn open_application(&mut self, message: &[u8]) -> anyhow::Result<(u32, Vec<u8>)> {
        let processed = self.process_message(message)?;
        let Sender::Member(leaf) = processed.sender().clone() else {
            return Err(anyhow!("application message from a non-member"));
        };
        match processed.into_content() {
            ProcessedMessageContent::ApplicationMessage(data) => Ok((leaf.u32(), data.into_bytes())),
            _ => Err(anyhow!("expected an application message")),
        }
    }

    pub fn epoch(&self) -> u64 {
        self.group.epoch().as_u64()
    }

    pub fn own_leaf(&self) -> u32 {
        self.group.own_leaf_index().u32()
    }

    pub fn is_active(&self) -> bool {
        self.group.is_active()
    }

    pub fn roster(&self) -> Vec<RosterMember> {
        self.group
            .members()
            .filter_map(|member| {
                let credential = BasicCredential::try_from(member.credential).ok()?;
                let identity = std::str::from_utf8(credential.identity()).ok()?.parse().ok()?;
                Some(RosterMember {
                    leaf: member.index.u32(),
                    identity,
                })
            })
            .collect()
    }

    /// Per-sender SFrame base keys for this epoch.
    ///
    /// The exporter yields one secret per epoch (RFC 9605 § 5.2); each sender's base key is then
    /// expanded from it with the sender's leaf index, so no two senders share key material even
    /// before the key id is bound in. The SFrame layer binds the key id as well; the per-sender
    /// expansion means sender separation does not rest on that alone.
    pub fn sframe_base_keys(&self) -> Vec<(u32, [u8; 32])> {
        let Ok(secret) = self.group.export_secret(
            self.provider.crypto(),
            Self::SFRAME_LABEL,
            b"",
            Self::SFRAME_KEY_LEN,
        ) else {
            return Vec::new();
        };
        let secret = Zeroizing::new(secret);
        let Ok(expander) = Hkdf::<Sha256>::from_prk(&secret) else {
            return Vec::new();
        };
        self.group
            .members()
            .filter_map(|member| {
                let leaf = member.index.u32();
                let mut info = Self::SENDER_LABEL.to_vec();
                info.extend_from_slice(&leaf.to_be_bytes());
                let mut key = [0u8; 32];
                expander.expand(&info, &mut key).ok()?;
                Some((leaf, key))
            })
            .collect()
    }

    pub fn safety_code(&self) -> SafetyCode {
        SafetyCode::new(self.group.epoch_authenticator().as_slice())
    }

    fn process_message(
        &mut self,
        message: &[u8],
    ) -> anyhow::Result<openmls::prelude::ProcessedMessage> {
        let message = MlsMessageIn::tls_deserialize_exact_bytes(message)?;
        let protocol = message.try_into_protocol_message()?;
        Ok(self.group.process_message(&self.provider, protocol)?)
    }

    /// Zeroizes every secret this member's OpenMLS store holds. Dropping does this too; after it
    /// the group can no longer be used.
    pub fn wipe(&mut self) {
        StorageWiper::wipe(&self.provider);
    }

    /// Whether the in-memory store holds anything, so a caller can confirm a wipe.
    pub fn storage_is_empty(&self) -> bool {
        self.provider
            .storage()
            .values
            .read()
            .map(|values| values.is_empty())
            .unwrap_or(false)
    }

    fn identity_material(
        identity: &common::PlayerIdentity,
    ) -> anyhow::Result<(Provider, SignatureKeyPair, CredentialWithKey)> {
        let provider = Provider::new()?;
        let signer = SignatureKeyPair::new(Self::SUITE.signature_algorithm())?;
        signer.store(provider.storage())?;
        let credential = CredentialWithKey {
            credential: BasicCredential::new(identity.to_string().into_bytes()).into(),
            signature_key: signer.public().into(),
        };
        Ok((provider, signer, credential))
    }
}

impl Drop for MlsGroupState {
    fn drop(&mut self) {
        self.wipe();
    }
}
