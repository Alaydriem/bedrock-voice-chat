use anyhow::anyhow;
use openmls::prelude::{
    MlsGroupJoinConfig, MlsMessageBodyIn, MlsMessageIn, StagedWelcome,
    tls_codec::DeserializeBytes,
};
use openmls_basic_credential::SignatureKeyPair;
use openmls_libcrux_crypto::Provider;

use super::{MlsGroupState, StagedJoin, StorageWiper};

/// A joiner holding the private half of the KeyPackage it offered, waiting for its Welcome.
///
/// Joining is two steps, so the joiner can check who signed the Welcome and which group it is for
/// before it takes the group's secrets: `stage` reads the Welcome, `finish` enters the group.
/// Whatever is left when this is dropped — a join that never happened — has its store zeroized,
/// since it holds the KeyPackage's private keys.
pub struct PendingJoin {
    provider: Option<Provider>,
    signer: Option<SignatureKeyPair>,
}

impl PendingJoin {
    pub(super) fn new(provider: Provider, signer: SignatureKeyPair) -> Self {
        Self {
            provider: Some(provider),
            signer: Some(signer),
        }
    }

    pub fn stage(&self, welcome: &[u8]) -> anyhow::Result<StagedJoin> {
        let message = MlsMessageIn::tls_deserialize_exact_bytes(welcome)?;
        let MlsMessageBodyIn::Welcome(welcome) = message.extract() else {
            return Err(anyhow!("not a Welcome"));
        };
        let Some(provider) = self.provider.as_ref() else {
            return Err(anyhow!("pending join already used"));
        };
        let config = MlsGroupJoinConfig::builder()
            .use_ratchet_tree_extension(true)
            .build();
        let staged = StagedWelcome::new_from_welcome(provider, &config, welcome, None)?;
        Ok(StagedJoin { staged })
    }

    pub fn finish(mut self, staged: StagedJoin) -> anyhow::Result<MlsGroupState> {
        let (Some(provider), Some(signer)) = (self.provider.take(), self.signer.take()) else {
            return Err(anyhow!("pending join already used"));
        };
        match staged.staged.into_group(&provider) {
            Ok(group) => Ok(MlsGroupState::from_parts(provider, signer, group)),
            Err(e) => {
                StorageWiper::wipe(&provider);
                Err(e.into())
            }
        }
    }

    /// Stages and enters in one step, for a caller that checks nothing in between.
    pub fn join(self, welcome: &[u8]) -> anyhow::Result<MlsGroupState> {
        let staged = self.stage(welcome)?;
        self.finish(staged)
    }
}

impl Drop for PendingJoin {
    fn drop(&mut self) {
        if let Some(provider) = self.provider.as_ref() {
            StorageWiper::wipe(provider);
        }
    }
}
