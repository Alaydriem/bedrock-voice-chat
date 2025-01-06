use common::ncryptflib::rocket::base64;
use keytar::*;
use anyhow::anyhow;

const BVC_SERVICE_NAME: &'static str = "BEDROCK_VOICE_CHAT";

/// Sets a raw credential by key and value
#[tauri::command]
pub(crate) fn set_credential_raw(key: &str, value: &str) -> bool {
    match set_password(BVC_SERVICE_NAME, key, value) {
        Ok(_) => true,
        Err(e) => {
            tracing::error!("{}", e.to_string());
            false
        }
    }
}

/// Retrieves a raw credential by key
#[tauri::command]
pub(crate) fn get_credential_raw(key: &str) -> Result<String, bool> {
    match get_password(BVC_SERVICE_NAME, key) {
        Ok(password) =>
            match password.success {
                true => Ok(password.password),
                false => Err(false),
            }
        Err(e) => {
            tracing::error!("{}", e.to_string());
            Err(false)
        }
    }
}

/// Deletes a raw credential by a key
#[tauri::command]
pub(crate) fn del_credential_raw(key: &str) -> bool {
    match delete_password(BVC_SERVICE_NAME, key) {
        Ok(_) => true,
        Err(e) => {
            tracing::error!("{}", e.to_string());
            false
        }
    }
}

/// Sets a credential for the current host key
#[tauri::command]
pub(crate) fn set_credential(value: serde_json::Value) -> bool {
    match get_active_host_key() {
        Ok(host_key) =>
            match serde_json::to_string(&value) {
                Ok(s) => set_credential_raw(host_key.as_str(), s.as_str()),
                Err(_) => false,
            }
        Err(_) => false,
    }
}

/// Retrieves a single credential from the given host key json
#[tauri::command]
pub(crate) fn get_credential(key: &str) -> Result<String, bool> {
    match get_active_host_key() {
        Ok(host_key) =>
            match get_credential_raw(host_key.as_str()) {
                Ok(s) =>
                    match serde_json::from_str::<serde_json::Value>(&s) {
                        Ok(v) =>
                            match v.get(key) {
                                Some(v) => Ok(v.as_str().unwrap().to_string()),
                                None => Err(false),
                            }
                        Err(_) => Err(false),
                    }
                Err(_) => Err(false),
            }
        Err(_) => Err(false),
    }
}

pub(crate) fn update_server_list(key: &str) -> bool {
    let mut servers = match get_credential_raw("server_list") {
        Ok(servers) =>
            match serde_json::from_str::<Vec<String>>(&servers) {
                Ok(servers) => servers,
                Err(_) => Vec::new(),
            }
        Err(_) => Vec::new(),
    };

    match servers.contains(&key.to_string()) {
        true => {
            let index = servers
                .iter()
                .position(|x| *x == key)
                .unwrap();
            servers.remove(index);
        }
        false => {
            servers.push(key.to_string());
        }
    }

    match serde_json::to_string(&servers) {
        Ok(json) => set_credential_raw("server_list", &json),
        Err(_) => false,
    }
}

/// Returns the active host key
fn get_active_host_key() -> Result<String, anyhow::Error> {
    match get_credential_raw("current_server") {
        Ok(password) => { Ok(format!("{}", base64::encode(password))) }
        Err(e) => {
            tracing::error!("{}", e.to_string());
            Err(anyhow!("Could not retrieve credential"))
        }
    }
}

#[cfg(test)]
mod credential_tests {
    use super::*;

    #[test]
    fn test_credentials() {
        let payload =
            serde_json::json!({
            "Hello": "World",
            "One": "Person"
        });

        assert_eq!(true, set_credential_raw("current_server", "example.bvc.alaydriem.com"));
        assert_eq!(true, set_credential(payload.clone()));

        match get_credential("Hello") {
            Ok(s) => {
                assert_eq!(s, "World".to_string());
            }
            Err(_) => assert!(true == false),
        }
        assert!(del_credential_raw(get_active_host_key().unwrap().as_str()));
        assert!(del_credential_raw("current_server"));
    }
}
