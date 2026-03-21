use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemProps {
    pub is_debug: bool,
    pub os_name: String,
    pub os_version: String,
    pub app_version: String,
    pub app_build_number: String,
    pub sdk_version: String,
}

impl SystemProps {
    pub fn new(app_version: String) -> Self {
        Self {
            is_debug: cfg!(debug_assertions),
            os_name: std::env::consts::OS.to_string(),
            os_version: String::new(),
            app_version,
            app_build_number: String::new(),
            sdk_version: "bvc@1.0.0".to_string(),
        }
    }
}
