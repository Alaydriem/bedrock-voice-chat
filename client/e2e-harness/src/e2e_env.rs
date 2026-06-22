use bvc_client_lib::testkit::connect::ConnectConfig;

// Reads the four optional BVC_E2E_* variables. The connect sequence only runs
// when a login code is present; the standalone smoke leaves it unset.
pub struct E2eEnv;

impl E2eEnv {
    pub fn connect_config() -> Option<ConnectConfig> {
        let code = std::env::var("BVC_E2E_CODE")
            .ok()
            .filter(|s| !s.is_empty())?;
        Some(ConnectConfig {
            server: std::env::var("BVC_E2E_SERVER").unwrap_or_default(),
            gamertag: std::env::var("BVC_E2E_GAMERTAG").unwrap_or_default(),
            code,
            channel: std::env::var("BVC_E2E_CHANNEL")
                .ok()
                .filter(|s| !s.is_empty()),
            channel_id: std::env::var("BVC_E2E_CHANNEL_ID")
                .ok()
                .filter(|s| !s.is_empty()),
        })
    }
}
