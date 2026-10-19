use anyhow::anyhow;
use openmls::prelude::{
    MlsGroupJoinConfig, MlsMessageBodyIn, MlsMessageIn, StagedWelcome,
    tls_codec::DeserializeBytes,
};
use openmls_basic_credential::SignatureKeyPair;
use openmls_libcrux_crypto::Provider;

use super::{MlsGroupState, StorageWiper};

/// A joiner holding the private half of the KeyPackage it offered, waiting for its Welcome.
///
/// The fields are taken by `join`; whatever is left when this is dropped — a join that never
/// happened — has its store zeroized, since it holds the KeyPackage's private keys.
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

    pub fn join(mut self, welcome: &[u8]) -> anyhow::Result<MlsGroupState> {
        let message = MlsMessageIn::tls_deserialize_exact_bytes(welcome)?;
        let MlsMessageBodyIn::Welcome(welcome) = message.extract() else {
            return Err(anyhow!("not a Welcome"));
        };
        let (Some(provider), Some(signer)) = (self.provider.take(), self.signer.take()) else {
            return Err(anyhow!("pending join already used"));
        };
        let config = MlsGroupJoinConfig::builder()
            .use_ratchet_tree_extension(true)
            .build();
        let joined = (|| -> anyhow::Result<openmls::prelude::MlsGroup> {
            Ok(StagedWelcome::new_from_welcome(&provider, &config, welcome, None)?
                .into_group(&provider)?)
        })();
        match joined {
            Ok(group) => Ok(MlsGroupState::from_parts(provider, signer, group)),
            Err(e) => {
                StorageWiper::wipe(&provider);
                Err(e)
            }
        }
    }
}

impl Drop for PendingJoin {
    fn drop(&mut self) {
        if let Some(provider) = self.provider.as_ref() {
            StorageWiper::wipe(provider);
        }
    }
}
