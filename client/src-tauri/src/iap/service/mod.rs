use crate::iap::provider::{EntitlementProvider, EntitlementProviderType};

pub struct EntitlementService {
    provider: EntitlementProviderType,
}

impl EntitlementService {
    pub fn new(provider: EntitlementProviderType) -> Self {
        Self { provider }
    }

    pub fn require_entitlement(&self) -> Result<(), String> {
        if self.is_entitled() {
            Ok(())
        } else {
            Err("Bedrock features require a purchase. Please buy the Bedrock add-on.".to_string())
        }
    }

    pub fn is_entitled(&self) -> bool {
        self.provider.is_entitled()
    }

    // Reserved for real IAP verification; not yet wired to a command.
    #[allow(dead_code)]
    pub fn check_and_refresh(&self) -> Result<bool, String> {
        self.provider.check_and_refresh()
    }

    #[allow(dead_code)]
    pub fn purchase(&self) -> Result<bool, String> {
        self.provider.purchase()
    }
}
