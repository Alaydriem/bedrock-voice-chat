use serde_json::json;

use crate::lxd_config::LxdConfig;
use crate::target_spec::TargetSpec;

/// Result of an in-container command run via the LXD `exec` API (record-output
/// mode): the process exit code plus its captured stdout/stderr.
pub struct ExecResult {
    pub exit_code: i64,
    pub stdout: String,
    pub stderr: String,
}

/// A thin async client for one LXD daemon's HTTPS REST API (`/1.0/...`),
/// authenticated with a client cert trusted on that daemon. Covers exactly the
/// swarm's needs: launch an ephemeral container, push files, exec, pull files,
/// stop. No `lxc` CLI dependency — works from any OS.
///
/// NOTE: endpoint/operation shapes follow the LXD 1.0 API; validate against the
/// real daemon on first run and adjust any version-specific field nesting.
pub struct LxdClient {
    client: reqwest::Client,
    base: String,
    image: String,
    image_protocol: String,
    image_server: String,
}

impl LxdClient {
    pub fn new(lxd: &LxdConfig, target: &TargetSpec) -> Result<Self, anyhow::Error> {
        let cert = std::fs::read(&lxd.client_cert)
            .map_err(|e| anyhow::anyhow!("reading lxd client_cert {}: {}", lxd.client_cert, e))?;
        let key = std::fs::read(&lxd.client_key)
            .map_err(|e| anyhow::anyhow!("reading lxd client_key {}: {}", lxd.client_key, e))?;
        let mut identity_pem = cert;
        identity_pem.extend_from_slice(b"\n");
        identity_pem.extend_from_slice(&key);
        let identity = reqwest::Identity::from_pem(&identity_pem)
            .map_err(|e| anyhow::anyhow!("building lxd client identity: {}", e))?;

        let mut builder = reqwest::Client::builder().use_rustls_tls().identity(identity);
        match &target.server_cert {
            Some(path) => {
                let ca = std::fs::read(path)
                    .map_err(|e| anyhow::anyhow!("reading lxd server_cert {}: {}", path, e))?;
                let ca_cert = reqwest::Certificate::from_pem(&ca)
                    .map_err(|e| anyhow::anyhow!("parsing lxd server_cert {}: {}", path, e))?;
                builder = builder.add_root_certificate(ca_cert);
            }
            // LXD daemons use a self-signed cert pinned by fingerprint; on a LAN
            // we accept it. Provide server_cert to pin instead.
            None => builder = builder.danger_accept_invalid_certs(true),
        }

        let client = builder
            .build()
            .map_err(|e| anyhow::anyhow!("building lxd client for {}: {}", target.endpoint, e))?;

        Ok(Self {
            client,
            base: target.endpoint.trim_end_matches('/').to_string(),
            image: lxd.image.clone(),
            image_protocol: lxd.image_protocol.clone(),
            image_server: lxd.image_server.clone(),
        })
    }

    // Blocks (server-side long-poll) until an async operation finishes; returns
    // the operation object's `metadata` on success, or its error on failure.
    async fn wait_operation(&self, op_path: &str) -> Result<serde_json::Value, anyhow::Error> {
        let url = format!("{}{}/wait", self.base, op_path);
        let resp: serde_json::Value = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("waiting op {}: {}", op_path, e))?
            .json()
            .await
            .map_err(|e| anyhow::anyhow!("decoding op {}: {}", op_path, e))?;

        let op = &resp["metadata"];
        let status = op["status"].as_str().unwrap_or("Unknown");
        if status == "Success" {
            Ok(op["metadata"].clone())
        } else {
            Err(anyhow::anyhow!(
                "op {} {}: {}",
                op_path,
                status,
                op["err"].as_str().unwrap_or("")
            ))
        }
    }

    // Extracts the "/1.0/operations/<id>" path from an async POST/PUT response.
    fn operation_path(resp: &serde_json::Value) -> Result<String, anyhow::Error> {
        resp["operation"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow::anyhow!("response is not an async operation: {}", resp))
    }

    /// Create an ephemeral container from the configured image and start it.
    pub async fn launch(&self, name: &str, cloud_init: Option<&str>) -> Result<(), anyhow::Error> {
        let mut config = serde_json::Map::new();
        if let Some(user_data) = cloud_init {
            config.insert("user.user-data".to_string(), json!(user_data));
        }

        let body = json!({
            "name": name,
            "type": "container",
            "ephemeral": true,
            "config": config,
            "source": {
                "type": "image",
                "protocol": self.image_protocol,
                "server": self.image_server,
                "alias": self.image,
            },
        });

        let resp: serde_json::Value = self
            .client
            .post(format!("{}/1.0/instances", self.base))
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("create container {}: {}", name, e))?
            .json()
            .await
            .map_err(|e| anyhow::anyhow!("create container {} decode: {}", name, e))?;
        self.wait_operation(&Self::operation_path(&resp)?).await?;

        self.set_state(name, "start").await
    }

    async fn set_state(&self, name: &str, action: &str) -> Result<(), anyhow::Error> {
        let resp: serde_json::Value = self
            .client
            .put(format!("{}/1.0/instances/{}/state", self.base, name))
            .json(&json!({ "action": action, "timeout": 60, "force": true }))
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("{} container {}: {}", action, name, e))?
            .json()
            .await
            .map_err(|e| anyhow::anyhow!("{} container {} decode: {}", action, name, e))?;
        self.wait_operation(&Self::operation_path(&resp)?).await?;
        Ok(())
    }

    /// Stop the container. Ephemeral containers auto-delete on stop.
    pub async fn stop(&self, name: &str) -> Result<(), anyhow::Error> {
        self.set_state(name, "stop").await
    }

    /// Push raw bytes to a path inside the container.
    pub async fn push_file(
        &self,
        name: &str,
        remote_path: &str,
        data: Vec<u8>,
        mode: u32,
    ) -> Result<(), anyhow::Error> {
        let resp = self
            .client
            .post(format!("{}/1.0/instances/{}/files", self.base, name))
            .query(&[("path", remote_path)])
            .header("X-LXD-type", "file")
            .header("X-LXD-mode", format!("{:o}", mode))
            .header("X-LXD-uid", "0")
            .header("X-LXD-gid", "0")
            .body(data)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("push {} to {}: {}", remote_path, name, e))?;
        if resp.status().is_success() {
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "push {} to {} returned {}",
                remote_path,
                name,
                resp.status().as_u16()
            ))
        }
    }

    /// Run a command in the container, waiting for completion, and return its
    /// exit code and captured output. No client-side timeout, so long-running
    /// agent runs are fine.
    pub async fn exec(&self, name: &str, command: Vec<String>) -> Result<ExecResult, anyhow::Error> {
        let resp: serde_json::Value = self
            .client
            .post(format!("{}/1.0/instances/{}/exec", self.base, name))
            .json(&json!({
                "command": command,
                "wait-for-websocket": false,
                "record-output": true,
                "interactive": false,
                "environment": {},
            }))
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("exec on {}: {}", name, e))?
            .json()
            .await
            .map_err(|e| anyhow::anyhow!("exec on {} decode: {}", name, e))?;

        let meta = self.wait_operation(&Self::operation_path(&resp)?).await?;
        let exit_code = meta["return"].as_i64().unwrap_or(-1);

        let stdout = match meta["output"]["1"].as_str() {
            Some(path) => String::from_utf8_lossy(&self.get_log(path).await?).into_owned(),
            None => String::new(),
        };
        let stderr = match meta["output"]["2"].as_str() {
            Some(path) => String::from_utf8_lossy(&self.get_log(path).await?).into_owned(),
            None => String::new(),
        };

        Ok(ExecResult {
            exit_code,
            stdout,
            stderr,
        })
    }

    async fn get_log(&self, log_path: &str) -> Result<Vec<u8>, anyhow::Error> {
        Ok(self
            .client
            .get(format!("{}{}", self.base, log_path))
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("fetch log {}: {}", log_path, e))?
            .bytes()
            .await
            .map_err(|e| anyhow::anyhow!("read log {}: {}", log_path, e))?
            .to_vec())
    }
}
