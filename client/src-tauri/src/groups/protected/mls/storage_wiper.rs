use openmls_libcrux_crypto::Provider;
use openmls_traits::OpenMlsProvider;
use zeroize::Zeroize;

/// Zeroizes everything OpenMLS keeps in a provider's in-memory store: epoch secrets, the secret
/// tree, HPKE and signature private keys. The store frees its values without wiping them, so a
/// group's secrets would otherwise linger in freed memory after the group is dropped.
pub struct StorageWiper;

impl StorageWiper {
    pub fn wipe(provider: &Provider) {
        let Ok(mut values) = provider.storage().values.write() else {
            return;
        };
        for (mut key, mut value) in values.drain() {
            key.zeroize();
            value.zeroize();
        }
    }
}
