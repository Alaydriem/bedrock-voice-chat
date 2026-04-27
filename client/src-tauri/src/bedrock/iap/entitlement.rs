use std::sync::atomic::{AtomicBool, Ordering};

pub struct BedrockEntitlementCheck {
    entitled: AtomicBool,
    #[allow(dead_code)]
    app_handle: tauri::AppHandle,
}

impl BedrockEntitlementCheck {
    pub fn new(app_handle: tauri::AppHandle) -> Self {
        Self {
            entitled: AtomicBool::new(true),
            app_handle,
        }
    }

    pub fn require_entitlement(&self) -> Result<(), String> {
        if self.is_entitled() {
            Ok(())
        } else {
            Err("Bedrock features require a purchase. Please buy the Bedrock add-on.".to_string())
        }
    }

    pub fn is_entitled(&self) -> bool {
        self.entitled.load(Ordering::SeqCst)
    }

    // TODO: Replace with actual IAP verification
    pub fn check_and_refresh(&self) -> Result<bool, String> {
        let is_purchased = true;

        self.entitled.store(is_purchased, Ordering::SeqCst);
        Ok(is_purchased)
    }

    pub fn purchase(&self) -> Result<bool, String> {
        Err("IAP purchase flow not yet implemented for this platform.".to_string())
    }
}
