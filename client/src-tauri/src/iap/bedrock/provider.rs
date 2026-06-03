use std::sync::atomic::{AtomicBool, Ordering};

use crate::iap::provider::EntitlementProvider;

pub struct Provider {
    entitled: AtomicBool,
    #[allow(dead_code)]
    app_handle: tauri::AppHandle,
}

impl Provider {
    pub fn new(app_handle: tauri::AppHandle) -> Self {
        Self {
            entitled: AtomicBool::new(true),
            app_handle,
        }
    }
}

impl EntitlementProvider for Provider {
    fn is_entitled(&self) -> bool {
        self.entitled.load(Ordering::SeqCst)
    }

    // TODO: Replace with actual IAP verification
    fn check_and_refresh(&self) -> Result<bool, String> {
        let is_purchased = true;

        self.entitled.store(is_purchased, Ordering::SeqCst);
        Ok(is_purchased)
    }

    fn purchase(&self) -> Result<bool, String> {
        Err("IAP purchase flow not yet implemented for this platform.".to_string())
    }
}
