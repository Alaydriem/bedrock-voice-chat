use anyhow::anyhow;
use common::Game;
use common::structs::channel::{ChannelEvent, ChannelEvents};
use tauri::async_runtime::Mutex;
use tauri::{AppHandle, Manager};
use tauri_plugin_curia::curia;
use tauri_plugin_store::StoreExt;
use websocket_types::GroupData;

use zeroize::Zeroize;

use super::GroupResolution;
use super::protected::{
    GroupKeyService, GroupPassword, KeySocket, ProtectedGroupNotifier, ProtectedGroupRunner,
    SystemClock,
};
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
        // Keys go first and protection last: between the two the client is silent, never plain.
        self.stop_protected().await;
        let left = self.leave_current().await;
        self.release_protected().await;
        left
    }

    async fn leave_current(&self) -> Result<GroupData, anyhow::Error> {
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

    /// Creates an end-to-end encrypted group and joins it as its first member.
    ///
    /// The password is checked and moved into a zeroizing holder before anything goes on the
    /// wire, and never leaves this process.
    pub async fn create_protected(
        &self,
        name: String,
        mut password: String,
    ) -> Result<GroupData, anyhow::Error> {
        let password = Self::take_password(&mut password)?;
        let api = self.api().await?;
        let id = api
            .create_protected_channel(name.clone())
            .await
            .map_err(|e| anyhow!(e))?;
        self.enter_protected(&api, id.clone(), password, true).await?;

        Ok(GroupData {
            in_group: true,
            id: Some(id),
            name: Some(name),
        })
    }

    /// Joins an end-to-end encrypted group. Membership is not complete until a member admits
    /// this client; the outcome arrives as a `protected-group:joined` or `protected-group:lost`
    /// event.
    pub async fn join_protected(
        &self,
        id: String,
        mut password: String,
    ) -> Result<GroupData, anyhow::Error> {
        let password = Self::take_password(&mut password)?;
        let api = self.api().await?;
        self.enter_protected(&api, id.clone(), password, false).await?;

        Ok(GroupData {
            in_group: true,
            id: Some(id),
            name: None,
        })
    }

    /// Stops this client's protected group, if it holds one, and wipes its keys. The gate stays
    /// protected, so the microphone is silent until `release_protected` — which the caller sends
    /// only once it has left the channel.
    pub async fn stop_protected(&self) {
        self.protected_slot().await.stop();
    }

    /// Lets audio flow unsealed again. Called after the protected channel has been left: before
    /// that, unsealed audio would reach the server while it still routes this client as a member.
    pub async fn release_protected(&self) {
        self.protected_slot().await.gate().set_protected(false);
    }

    /// Re-asserts this client's membership of its protected group's channel. A voice-link
    /// reconnect drops channel membership on the server, while the group's keys and epoch survive
    /// on the client; without this the client goes on sealing into a channel it is no longer in.
    pub async fn rejoin_protected(&self) -> Result<(), anyhow::Error> {
        let Some(id) = self.protected_slot().await.channel() else {
            return Ok(());
        };
        self.api()
            .await?
            .join_protected_channel(id)
            .await
            .map(|_| ())
            .map_err(|e| anyhow!(e))
    }

    /// Protects the gate, joins the channel's routing, then starts the group. Protection comes
    /// first so no unsealed frame is sent once the server routes this client as a member; a start
    /// that fails leaves the channel before protection is lifted.
    async fn enter_protected(
        &self,
        api: &Api,
        id: String,
        password: GroupPassword,
        create: bool,
    ) -> Result<(), anyhow::Error> {
        let slot = self.protected_slot().await;
        slot.stop();
        slot.gate().set_protected(true);

        let entered = match api.join_protected_channel(id.clone()).await {
            Ok(_) => self.start_protected(api, id.clone(), password, create).await,
            Err(e) => {
                slot.gate().set_protected(false);
                return Err(anyhow!(e));
            }
        };
        if let Err(e) = entered {
            if let Err(leave) = self.send(&id, ChannelEvents::Leave).await {
                curia::warn!("could not leave a protected group that failed to start", {
                    channel_id: id.clone(),
                    error: leave.to_string(),
                });
            }
            slot.gate().set_protected(false);
            return Err(e);
        }
        Ok(())
    }

    fn take_password(password: &mut String) -> Result<GroupPassword, anyhow::Error> {
        let checked = GroupPassword::new(password);
        password.zeroize();
        Ok(checked?)
    }

    async fn start_protected(
        &self,
        api: &Api,
        id: String,
        password: GroupPassword,
        create: bool,
    ) -> Result<(), anyhow::Error> {
        let identity: common::PlayerIdentity = self.current_player().await?.parse()?;
        let (ca_cert, pem) = api.tls_material();
        let (socket_tx, socket_events) =
            KeySocket::connect(api.endpoint(), ca_cert, pem, &id).await?;
        let (event_tx, event_rx) = tokio::sync::mpsc::channel(64);
        let slot = self.protected_slot().await;
        let gate = slot.gate();
        let id = id.clone();
        let service = if create {
            GroupKeyService::create(id.clone(), identity, password, SystemClock, socket_tx, event_tx, gate.clone())?
        } else {
            GroupKeyService::join(id.clone(), identity, password, SystemClock, socket_tx, event_tx, gate.clone())?
        };

        let (stop_tx, stop) = tokio::sync::oneshot::channel();
        slot.install(stop_tx, id);
        let app = self.app_handle.clone();
        // Only tells the webview. A lost group leaves the gate protected and keyless, so the
        // microphone is silent until the user leaves the channel; it never falls back to plaintext
        // on its own, which is exactly what a hostile server closing the socket would want.
        tauri::async_runtime::spawn(async move {
            ProtectedGroupRunner::run(service, socket_events, event_rx, stop, move |event| {
                ProtectedGroupNotifier::notify(&app, &event)
            })
            .await;
        });
        Ok(())
    }

    async fn protected_slot(&self) -> std::sync::Arc<crate::groups::protected::ProtectedGroupSlot> {
        self.app_handle
            .state::<std::sync::Arc<crate::groups::protected::ProtectedGroupSlot>>()
            .inner()
            .clone()
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

        Ok(ControlActionsManager::canonical(&bare, &self.game().await))
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
