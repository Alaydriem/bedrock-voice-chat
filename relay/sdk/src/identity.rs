use std::sync::Arc;

use bvc_relay::node::NodeIdentity;

use crate::error::SdkError;

// This peer's own identity, readable before any session exists.
//
// A consumer has to hand its peerlink to the operator of the BVC server it wants to
// reach, and that server has to name it in `config.hcl` before the session it
// authorizes can be opened. Deriving the link from `BvcPeer` would make that
// circular: opening a peer requires the far side's link, which the operator cannot
// issue until they have ours.
//
// The key is persisted on first use, so the link an operator pastes stays valid
// across restarts.
#[derive(uniffi::Object)]
pub struct BvcIdentity {
    identity: NodeIdentity,
}

#[uniffi::export]
impl BvcIdentity {
    #[uniffi::constructor]
    pub fn open(node_dir: String) -> Result<Arc<Self>, SdkError> {
        let identity = NodeIdentity::load_or_create(&node_dir).map_err(|e| SdkError::Open {
            reason: e.to_string(),
        })?;

        Ok(Arc::new(Self { identity }))
    }

    // What the operator pastes into the far side's `config.hcl`.
    pub fn peerlink(&self) -> Result<String, SdkError> {
        self.identity
            .peerlink()
            .map_err(|e| SdkError::PeerLink {
                reason: e.to_string(),
            })
    }

    pub fn node_id(&self) -> String {
        self.identity.node_id().to_string()
    }
}
