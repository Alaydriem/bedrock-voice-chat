pub mod address;
pub mod resolved;
pub mod saved_entry;

pub use address::ResolvedAddress;
pub use resolved::ResolvedTarget;
pub use saved_entry::SavedProxyEntry;

use tauri::{AppHandle, Manager};
use tauri_plugin_store::StoreExt;

use common::response::ApiConfigBedrockServer;
use common::structs::bedrock::RealmEntry;
use websocket_types::{ConnectTarget, ConnectTargetId, ConnectTargetSource};

/// Every world a controller may name, and the address behind each one.
pub struct BedrockTargetService {
    targets: Vec<ResolvedTarget>,
}

impl BedrockTargetService {
    /// Composes the three sources into one list.
    ///
    /// A saved entry and an advertised entry pointing at the same `host:port` are one world.
    /// The saved entry wins because it carries the name and protocol override the user chose;
    /// the webview's `sortedProxyServers` applies the same rule, so the two lists cannot
    /// disagree about what is on offer.
    pub fn new(
        saved: Vec<SavedProxyEntry>,
        advertised: Vec<ApiConfigBedrockServer>,
        realms: Vec<RealmEntry>,
    ) -> Self {
        let saved_addresses: Vec<(String, u16)> = saved
            .iter()
            .map(|entry| (entry.host.clone(), entry.port))
            .collect();

        let mut targets: Vec<ResolvedTarget> = saved
            .into_iter()
            .map(|entry| ResolvedTarget {
                id: ConnectTargetId::mint(ConnectTargetSource::Saved, &entry.id),
                name: entry.name,
                address: ResolvedAddress::Proxy {
                    host: entry.host,
                    port: entry.port,
                    protocol_version: entry.protocol_version,
                },
            })
            .collect();

        targets.extend(
            advertised
                .into_iter()
                .filter(|server| {
                    !saved_addresses
                        .iter()
                        .any(|(host, port)| host == &server.host && *port == server.port)
                })
                .map(|server| ResolvedTarget {
                    id: ConnectTargetId::mint(
                        ConnectTargetSource::Server,
                        &format!("{}:{}", server.host, server.port),
                    ),
                    name: server.name,
                    address: ResolvedAddress::Proxy {
                        host: server.host,
                        port: server.port,
                        protocol_version: server.protocol_version,
                    },
                }),
        );

        targets.extend(realms.into_iter().map(|world| ResolvedTarget {
            id: ConnectTargetId::mint(ConnectTargetSource::Realm, &world.id.to_string()),
            name: world.name,
            address: ResolvedAddress::Realm { realm_id: world.id },
        }));

        Self { targets }
    }

    pub fn targets(&self) -> Vec<ConnectTarget> {
        self.targets.iter().map(ResolvedTarget::to_wire).collect()
    }

    /// The entry an id names.
    ///
    /// Never by list position: listing and connecting are two calls, and an entry added
    /// between them would otherwise shift the operator onto a different world.
    pub fn resolve(&self, id: &str) -> Option<&ResolvedTarget> {
        ConnectTargetId::parse(id)?;
        self.targets.iter().find(|target| target.id == id)
    }

    /// The entry a running proxy session was started against.
    ///
    /// The session records a host and port, not an id, so this is how a live session gets the
    /// same name a listing would give it.
    pub fn resolve_by_address(&self, host: &str, port: u16) -> Option<&ResolvedTarget> {
        self.targets.iter().find(|target| {
            matches!(
                &target.address,
                ResolvedAddress::Proxy { host: h, port: p, .. } if h == host && *p == port
            )
        })
    }

    /// Saved and advertised proxies. No realms call.
    ///
    /// Naming a running proxy session does not need the realms list, and that call is a
    /// network round trip on a path the user is waiting on.
    pub async fn load_proxies(app_handle: &AppHandle) -> Result<Self, anyhow::Error> {
        Self::require_auth(app_handle).await?;
        Ok(Self::new(
            Self::saved_entries(app_handle),
            Self::advertised_entries(app_handle).await,
            Vec::new(),
        ))
    }

    /// Every world a controller may name.
    ///
    /// All-or-nothing. A realms listing that failed once returned the proxies alone, which
    /// named a set of worlds while quietly omitting another — a controller cannot tell that
    /// from a user who owns no realms.
    pub async fn load_all(app_handle: &AppHandle) -> Result<Self, anyhow::Error> {
        Self::require_auth(app_handle).await?;

        let api = {
            let state = app_handle
                .state::<tauri::async_runtime::Mutex<crate::bedrock::BedrockState>>();
            let state = state.lock().await;
            state.realms_api.clone()
        };
        let api = api.ok_or_else(|| anyhow::anyhow!(crate::bedrock::XBOX_AUTH_REQUIRED))?;
        let realms: Vec<RealmEntry> = api
            .list_worlds()
            .await?
            .into_iter()
            .map(|world| RealmEntry {
                id: world.id,
                name: world.name,
                motd: world.motd,
                state: world.state,
                owner_uuid: world.owner_uuid,
            })
            .collect();

        Ok(Self::new(
            Self::saved_entries(app_handle),
            Self::advertised_entries(app_handle).await,
            realms,
        ))
    }

    async fn require_auth(app_handle: &AppHandle) -> Result<(), anyhow::Error> {
        let state =
            app_handle.state::<tauri::async_runtime::Mutex<crate::bedrock::BedrockState>>();
        let state = state.lock().await;
        if state.auth_manager.is_none() {
            anyhow::bail!(crate::bedrock::XBOX_AUTH_REQUIRED);
        }
        Ok(())
    }

    fn saved_entries(app_handle: &AppHandle) -> Vec<SavedProxyEntry> {
        let Ok(store) = app_handle.store("store.json") else {
            return Vec::new();
        };
        let Some(raw) = store.get("bedrock_proxy_servers") else {
            return Vec::new();
        };

        match serde_json::from_value(raw) {
            Ok(entries) => entries,
            Err(e) => {
                log::warn!("Bedrock targets: saved proxies unreadable: {}", e);
                Vec::new()
            }
        }
    }

    /// Read through `Api::get_config`, which is a 30-second `FetchCache` keyed by endpoint.
    /// A listing is a cache hit in the normal case.
    async fn advertised_entries(app_handle: &AppHandle) -> Vec<ApiConfigBedrockServer> {
        let api = {
            let app_state = app_handle
                .state::<tauri::async_runtime::Mutex<crate::structs::app_state::AppState>>();
            let app = app_state.lock().await;
            app.api_client.clone()
        };

        let Some(api) = api else {
            return Vec::new();
        };

        match api.get_config().await {
            Ok(config) => config.bedrock.servers,
            Err(e) => {
                log::warn!("Bedrock targets: advertised servers unavailable: {}", e);
                Vec::new()
            }
        }
    }
}
