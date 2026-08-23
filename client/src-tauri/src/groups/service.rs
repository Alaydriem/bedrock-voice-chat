use anyhow::anyhow;
use common::Game;
use common::structs::channel::{ChannelEvent, ChannelEvents};
use tauri::async_runtime::Mutex;
use tauri::{AppHandle, Manager};
use tauri_plugin_store::StoreExt;
use websocket_types::GroupData;

use super::GroupResolution;
use crate::api::Api;
use crate::control::ControlActionsManager;
use crate::structs::app_state::AppState;

/// Group membership, driven from outside the window.
///
/// Goes through the HTTP channel API rather than asking the webview to act, so a controller works
/// against a client whose window is closed or unresponsive.
///
/// The join and leave events carry no player name — the server identifies the actor from the mTLS
/// client certificate — so the only identity work here is finding which group this client is in.
pub struct GroupService {
    app_handle: AppHandle,
}

impl GroupService {
    pub fn new(app_handle: AppHandle) -> Self {
        Self { app_handle }
    }

    pub async fn create(&self, name: String) -> Result<GroupData, anyhow::Error> {
        let api = self.api().await?;
        let id = api
            .create_channel(name.clone())
            .await
            .map_err(|e| anyhow!(e))?;

        // The HTTP create does not add the creator, unlike the QUIC control path. Without this a
        // create would leave the caller outside the group they just made, which reads as a no-op.
        self.send(&id, ChannelEvents::Join).await?;

        Ok(GroupData {
            in_group: true,
            id: Some(id),
            name: Some(name),
        })
    }

    pub async fn join(&self, name: String) -> Result<GroupData, anyhow::Error> {
        let api = self.api().await?;
        let channels = api.list_channels().await.map_err(|e| anyhow!(e))?;
        let (id, resolved) = {
            let channel = GroupResolution::by_name(&channels, &name)?;
            (channel.id(), channel.name.clone())
        };

        self.send(&id, ChannelEvents::Join).await?;

        Ok(GroupData {
            in_group: true,
            id: Some(id),
            name: Some(resolved),
        })
    }

    /// Leave whichever group this client is in.
    ///
    /// Stateless: nothing here tracks current membership, and the server already knows. Finding it
    /// costs one request, which is acceptable for a button press and cannot go stale the way a
    /// locally cached channel id would across a reconnect.
    pub async fn leave(&self) -> Result<GroupData, anyhow::Error> {
        let api = self.api().await?;
        let channels = api.list_channels().await.map_err(|e| anyhow!(e))?;
        let player = self.current_player().await?;

        let found = GroupResolution::containing(&channels, &player)
            .map(|channel| (channel.id(), channel.name.clone()));

        let Some((id, name)) = found else {
            return Ok(GroupData {
                in_group: false,
                id: None,
                name: None,
            });
        };

        self.send(&id, ChannelEvents::Leave).await?;

        Ok(GroupData {
            in_group: false,
            id: Some(id),
            name: Some(name),
        })
    }

    async fn send(&self, id: &str, event: ChannelEvents) -> Result<(), anyhow::Error> {
        let api = self.api().await?;
        let event = ChannelEvent {
            event,
            game: Some(self.game().await),
        };

        api.channel_event(id.to_string(), event)
            .await
            .map_err(|e| anyhow!(e))?;

        Ok(())
    }

    /// Private because `Api` is `pub(crate)`; returning it from a public method would be a private
    /// type in a public interface.
    async fn api(&self) -> Result<Api, anyhow::Error> {
        let state = self.app_handle.state::<Mutex<AppState>>();
        let state = state.lock().await;

        state
            .get_api_client()
            .map(|api| api.clone())
            .map_err(|e| anyhow!(e))
    }

    /// This client's identity in the form channel membership is keyed on.
    ///
    /// `current_player` is stored bare and membership is `game:gamertag`. Matched bare, the lookup
    /// finds nothing and a leave becomes a no-op that reports success.
    async fn current_player(&self) -> Result<String, anyhow::Error> {
        let store = self.app_handle.store("store.json")?;
        let bare = store
            .get("current_player")
            .and_then(|value| value.as_str().map(String::from))
            .ok_or_else(|| anyhow!("not signed in"))?;

        Ok(ControlActionsManager::canonical(
            &bare,
            &self.game().await,
        ))
    }

    async fn game(&self) -> Game {
        self.app_handle
            .store("store.json")
            .ok()
            .and_then(|store| store.get("active_game"))
            .and_then(|value| value.as_str().and_then(Game::from_tag))
            .unwrap_or(Game::Minecraft)
    }
}
