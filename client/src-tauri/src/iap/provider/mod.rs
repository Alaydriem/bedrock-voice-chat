use enum_dispatch::enum_dispatch;

use crate::iap::bedrock::Provider as BedrockProvider;

// Platform entitlement interface. Each platform/store backend implements this;
// feature modules (e.g. bedrock) consume it through EntitlementService.
#[enum_dispatch]
pub trait EntitlementProvider {
    fn is_entitled(&self) -> bool;

    // Reserved for real IAP verification; not yet wired to a command.
    #[allow(dead_code)]
    fn check_and_refresh(&self) -> Result<bool, String>;

    #[allow(dead_code)]
    fn purchase(&self) -> Result<bool, String>;
}

#[enum_dispatch(EntitlementProvider)]
pub enum EntitlementProviderType {
    Bedrock(BedrockProvider),
}
