use crate::audio::types::AudioDeviceType;
use crate::audio::{AudioStreamManager, RecordingManager};
use common::structs::audio::{MuteEvent, PlayerGainSettings, StreamEvent, VoiceRuntimeState};
use common::structs::keybinds::VoiceMode;
use log::info;
use std::sync::Arc;
use tauri::async_runtime::Mutex;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_store::StoreExt;

pub struct AudioActionsManager {
    app_handle: AppHandle,
}

impl AudioActionsManager {
    pub fn new(app_handle: AppHandle) -> Self {
        Self { app_handle }
    }

    /// Toggle mute for a device, emit `mute:{device}` event, return new mute status.
    pub async fn toggle_mute(&self, device: AudioDeviceType) -> bool {
        let asm = self.app_handle.state::<Mutex<AudioStreamManager>>();
        let mut asm = asm.lock().await;
        let _ = asm.toggle(&device, StreamEvent::Mute).await;
        let status = asm.mute_status(&device).await.unwrap_or(false);
        drop(asm);

        let mute_event = MuteEvent::from(&device);
        self.app_handle.emit(&mute_event.to_string(), status).ok();

        info!(
            "{} {}",
            mute_event,
            if status { "muted" } else { "unmuted" }
        );

        status
    }

    /// Query mute status without toggling.
    pub async fn is_muted(&self, device: AudioDeviceType) -> bool {
        let asm = self.app_handle.state::<Mutex<AudioStreamManager>>();
        let mut asm = asm.lock().await;
        asm.mute_status(&device).await.unwrap_or(false)
    }

    /// Toggle recording on/off. Returns new recording state.
    pub async fn toggle_recording(&self) -> Result<bool, anyhow::Error> {
        let recording_manager = self.app_handle.state::<Arc<Mutex<RecordingManager>>>();
        let mut manager = recording_manager.lock().await;

        if manager.is_recording() {
            manager.stop_recording().await?;
            Ok(false)
        } else {
            let current_player = self
                .app_handle
                .store("store.json")
                .ok()
                .and_then(|store| store.get("current_player"))
                .and_then(|v| v.as_str().map(String::from))
                .ok_or_else(|| anyhow::anyhow!("No current player"))?;

            manager.start_recording(current_player).await?;
            Ok(true)
        }
    }

    /// Drive a device to `desired`, flipping only if it differs — the check and the
    /// toggle happen under a single `AudioStreamManager` lock so an idempotent
    /// `set_mute(dev, true)` can't race the desktop-app / Stream-Deck toggle surfaces.
    pub async fn set_mute(&self, device: AudioDeviceType, desired: bool) -> bool {
        let asm = self.app_handle.state::<Mutex<AudioStreamManager>>();
        let mut asm = asm.lock().await;
        if asm.mute_status(&device).await.unwrap_or(false) != desired {
            let _ = asm.toggle(&device, StreamEvent::Mute).await;
        }
        let status = asm.mute_status(&device).await.unwrap_or(false);
        drop(asm);

        let mute_event = MuteEvent::from(&device);
        self.app_handle.emit(&mute_event.to_string(), status).ok();
        status
    }

    /// Deafen, and the mute that has to come with it.
    ///
    /// Hearing nobody while they can still hear you is a state people reach by accident
    /// and cannot detect from their own screen, so deafening drives the input too.
    /// Undeafening clears both, because the fix for "I cannot hear anyone" must not be a
    /// second button they have no reason to suspect.
    ///
    /// The pairing lives here rather than in each caller so the in-game action, the global
    /// hotkey and the app cannot disagree about what deafen means. Each leg still emits its
    /// own `mute:*` event, which is how every surface learns the resulting pair.
    /// Deafen, and put the microphone back where the voice mode says it belongs.
    ///
    /// Coming out of deafen used to open the input unconditionally. In push-to-talk that is
    /// the wrong resting state — the microphone is supposed to be shut until the button is
    /// held — so undeafening left it live, and the mic button, which reads the same flag,
    /// disagreed with the mode it was drawing.
    pub async fn set_deafened(&self, desired: bool) -> bool {
        self.set_mute(AudioDeviceType::OutputDevice, desired).await;
        self.set_mute(AudioDeviceType::InputDevice, desired || self.input_rests_muted().await)
            .await;
        desired
    }

    /// Whether an idle microphone belongs muted. True in push-to-talk, where only the hold
    /// opens it.
    pub async fn input_rests_muted(&self) -> bool {
        self.voice_mode_and_hold().await.0 == VoiceMode::PushToTalk
    }

    /// Adopt a server's recording policy, which the gate in `RecordingManager` reads
    /// on every attempt to arm.
    pub async fn set_recording_allowed(&self, allowed: bool) {
        if let Some(manager) = self
            .app_handle
            .try_state::<Arc<Mutex<RecordingManager>>>()
        {
            manager.lock().await.set_allowed(allowed);
        }
    }

    /// Drive recording to `desired`, starting/stopping only if it differs — the
    /// check and the transition happen under a single `RecordingManager` lock.
    pub async fn set_recording(&self, desired: bool) -> Result<bool, anyhow::Error> {
        let recording_manager = self.app_handle.state::<Arc<Mutex<RecordingManager>>>();
        let mut manager = recording_manager.lock().await;

        if manager.is_recording() == desired {
            return Ok(desired);
        }
        if desired {
            let current_player = self
                .app_handle
                .store("store.json")
                .ok()
                .and_then(|store| store.get("current_player"))
                .and_then(|v| v.as_str().map(String::from))
                .ok_or_else(|| anyhow::anyhow!("No current player"))?;
            manager.start_recording(current_player).await?;
        } else {
            manager.stop_recording().await?;
        }
        Ok(desired)
    }

    /// What the backend believes about this microphone, for the diagnostics readout.
    ///
    /// Read from the flag the capture stream itself consults and the mode the listener
    /// holds, so it reports the two copies that decide whether audio leaves this machine —
    /// not the third copy the UI keeps.
    pub async fn voice_runtime_state(&self) -> VoiceRuntimeState {
        let (voice_mode, ptt_active) = self.voice_mode_and_hold().await;
        VoiceRuntimeState {
            voice_mode,
            ptt_active,
            input_muted: self.is_muted(AudioDeviceType::InputDevice).await,
            output_muted: self.is_muted(AudioDeviceType::OutputDevice).await,
            recording: self.is_recording().await,
            recording_allowed: self.recording_allowed().await,
            jukebox_playing: self.jukebox_playing().await,
        }
    }

    /// The one place jukebox mute is written.
    ///
    /// Three copies have to move together: the metadata entry a rebuilt output stream restores
    /// from, `store.json` which survives a restart, and the atomic on the mixing path that the
    /// metadata write sets. Every surface that changes this — the settings pane, a WebSocket
    /// controller, the in-game panel — comes through here, so none of them can write a subset.
    pub async fn set_jukebox_muted(&self, desired: bool) -> Result<bool, anyhow::Error> {
        {
            let asm = self.app_handle.state::<Mutex<AudioStreamManager>>();
            let mut asm = asm.lock().await;
            asm.metadata(
                "jukebox_muted".to_string(),
                desired.to_string(),
                &AudioDeviceType::OutputDevice,
            )
            .await?;
        }

        let store = self.app_handle.store("store.json")?;
        store.set("jukebox_muted", desired);
        store.save()?;

        // The settings pane and the dashboard chip read the store, and neither polls this flag.
        // Without this they keep drawing the state from before the change for as long as they
        // stay mounted.
        let _ = self.app_handle.emit(
            crate::events::event::jukebox::JUKEBOX_MUTED_UPDATED,
            desired,
        );

        self.signal_jukebox_change();

        Ok(desired)
    }

    /// The one place jukebox level is written.
    ///
    /// The mirror of `set_jukebox_muted`, and it exists for the same reason: three copies have to
    /// move together — the metadata entry a rebuilt output stream restores from, `store.json`
    /// which survives a restart, and the atomic on the mixing path that the metadata write sets.
    /// Every surface that changes this comes through here, so none of them can write a subset.
    ///
    /// Returns the level actually applied, which is the requested one clamped to the ceiling.
    pub async fn set_jukebox_gain(&self, desired: f32) -> Result<f32, anyhow::Error> {
        // A non-finite level would reach the mixer as a NaN multiplier and silence the sink with
        // no error, so it is refused rather than clamped.
        if !desired.is_finite() {
            return Err(anyhow::anyhow!("jukebox gain must be finite"));
        }
        let desired = desired.clamp(0.0, PlayerGainSettings::MAX_GAIN);

        {
            let asm = self.app_handle.state::<Mutex<AudioStreamManager>>();
            let mut asm = asm.lock().await;
            asm.metadata(
                "jukebox_gain".to_string(),
                desired.to_string(),
                &AudioDeviceType::OutputDevice,
            )
            .await?;
        }

        let store = self.app_handle.store("store.json")?;
        store.set("jukebox_gain", desired);
        store.save()?;

        // The settings pane's slider and the dashboard chip read the store, and neither polls
        // this level. Without this they keep drawing the pre-change value for as long as they
        // stay mounted.
        let _ = self
            .app_handle
            .emit(crate::events::event::jukebox::JUKEBOX_GAIN_UPDATED, desired);

        self.signal_jukebox_change();

        Ok(desired)
    }

    /// Nudges the control-plane reporter so the in-game panel and the server's preference cache
    /// learn about a jukebox change.
    ///
    /// The jukebox rides the preference plane, so this is the preference signal rather than the
    /// self-state one. Without it a change made in the settings pane would never leave the
    /// desktop, and the panel would keep drawing the previous value until the 30s resync.
    fn signal_jukebox_change(&self) {
        if let Some(bus) = self.app_handle.try_state::<crate::control::ControlStateBus>() {
            bus.preferences();
        }
    }

    /// Flip it, for a control that cannot read the current value before it is pressed.
    pub async fn toggle_jukebox_muted(&self) -> Result<bool, anyhow::Error> {
        let current = self.jukebox_muted().await;
        self.set_jukebox_muted(!current).await
    }

    /// Whether jukebox music is muted, or false where no output stream is registered.
    ///
    /// Absent state rather than an unwrap, for the same reason as `is_recording`: this is read on
    /// a poll and on every state query, and must not panic where a stream has yet to be built.
    pub async fn jukebox_muted(&self) -> bool {
        let Some(asm) = self.app_handle.try_state::<Mutex<AudioStreamManager>>() else {
            return false;
        };
        let asm = asm.lock().await;
        asm.metadata_value("jukebox_muted", &AudioDeviceType::OutputDevice)
            .await
            .and_then(|value| value.parse::<bool>().ok())
            .unwrap_or(false)
    }

    /// The level jukebox music plays at, or unity where no output stream is registered.
    ///
    /// Reads the metadata entry rather than the atomic it seeds: that is the copy an
    /// output-stream rebuild restores from, so it is the one that stays true across a rebuild.
    ///
    /// Absent state rather than an unwrap, for the same reason as `jukebox_muted`: this is read
    /// on a poll and on every state query, and must not panic before a stream exists.
    pub async fn jukebox_gain(&self) -> f32 {
        let Some(asm) = self.app_handle.try_state::<Mutex<AudioStreamManager>>() else {
            return 1.0;
        };
        let asm = asm.lock().await;
        asm.metadata_value("jukebox_gain", &AudioDeviceType::OutputDevice)
            .await
            .and_then(|value| value.parse::<f32>().ok())
            .filter(|gain| gain.is_finite())
            .unwrap_or(1.0)
    }

    /// Whether jukebox frames are arriving, or false when no stream manager is registered.
    ///
    /// Absent state rather than an unwrap, for the same reason as `is_recording`: this is read on
    /// a poll and must not panic in the loop that keeps the self-state surfaces honest.
    async fn jukebox_playing(&self) -> bool {
        let Some(asm) = self.app_handle.try_state::<Mutex<AudioStreamManager>>() else {
            return false;
        };
        let asm = asm.lock().await;
        asm.peer_registry()
            .jukebox_playing(crate::diagnostics::PeerRegistry::JUKEBOX_PLAYING_WINDOW)
    }

    /// Whether a recording session is open, or false if nothing is holding one.
    ///
    /// Absent state rather than an unwrap: this is read on a poll, and a build without a
    /// recording manager registered must report "not recording" rather than panic in the
    /// loop that keeps every self-state surface honest.
    async fn is_recording(&self) -> bool {
        match self
            .app_handle
            .try_state::<Arc<Mutex<RecordingManager>>>()
        {
            Some(manager) => manager.lock().await.is_recording(),
            None => false,
        }
    }

    /// Whether the connected server permits recording, or true where no recording
    /// manager is registered — the same permissive absence as an unasked server.
    async fn recording_allowed(&self) -> bool {
        match self
            .app_handle
            .try_state::<Arc<Mutex<RecordingManager>>>()
        {
            Some(manager) => manager.lock().await.is_allowed(),
            None => true,
        }
    }

    async fn voice_mode_and_hold(&self) -> (VoiceMode, bool) {
        match self
            .app_handle
            .try_state::<Arc<crate::keybinds::KeybindListener>>()
        {
            Some(listener) => (listener.voice_mode().await, listener.is_ptt_held()),
            None => (VoiceMode::OpenMic, false),
        }
    }

    /// Query current mute, deafen, recording and voice-mode state as a DTO.
    ///
    /// The voice mode travels with the rest because it decides what the mute flag means: in
    /// push-to-talk `muted` is the resting state and only the hold clears it, so a
    /// controller that reads the flag without the mode draws a mute button that is on
    /// almost always and does nothing when pressed.
    ///
    /// The connected world is filled in here rather than by each caller. Sixteen surfaces
    /// broadcast through `broadcast_state`, and one that left it unset would tell every
    /// controller nothing is connected on the next mute or push-to-talk press.
    pub async fn query_state(&self) -> crate::websocket::StateData {
        let asm = self.app_handle.state::<Mutex<AudioStreamManager>>();
        let mut asm = asm.lock().await;
        let muted = asm
            .mute_status(&AudioDeviceType::InputDevice)
            .await
            .unwrap_or(false);
        let deafened = asm
            .mute_status(&AudioDeviceType::OutputDevice)
            .await
            .unwrap_or(false);
        drop(asm);

        let recording_manager = self.app_handle.state::<Arc<Mutex<RecordingManager>>>();
        let manager = recording_manager.lock().await;
        let recording = manager.is_recording();
        drop(manager);

        let (mode, ptt_active) = self.voice_mode_and_hold().await;
        let voice_mode = match mode {
            VoiceMode::PushToTalk => websocket_types::VoiceMode::PushToTalk,
            VoiceMode::OpenMic => websocket_types::VoiceMode::OpenMic,
        };

        crate::websocket::StateData {
            muted,
            deafened,
            recording,
            voice_mode,
            ptt_active,
            jukebox_muted: self.jukebox_muted().await,
            jukebox_gain: self.jukebox_gain().await,
            connection: self.active_connection().await,
        }
    }

    /// The world a session is running against, read from the bedrock state that owns it.
    ///
    /// Takes the `BedrockState` lock. `tauri::async_runtime::Mutex` is not reentrant, so a
    /// caller already holding that lock must release it before broadcasting — see
    /// `BedrockConnector::start_proxy`.
    #[cfg(feature = "bedrock-protocol")]
    async fn active_connection(&self) -> Option<websocket_types::ActiveConnection> {
        let state = self
            .app_handle
            .try_state::<tauri::async_runtime::Mutex<crate::bedrock::BedrockState>>()?;
        let state = state.lock().await;
        state.active_connection.clone()
    }

    #[cfg(not(feature = "bedrock-protocol"))]
    async fn active_connection(&self) -> Option<websocket_types::ActiveConnection> {
        None
    }

    /// Query state and broadcast to all WS clients.
    pub async fn broadcast_state(&self) {
        let state = self.query_state().await;
        let broadcaster = self
            .app_handle
            .state::<crate::websocket::WebSocketBroadcaster>();
        broadcaster.broadcast_state(state);

        // Every mute/deafen/record surface funnels through here; nudge the
        // control-plane reporter so the server cache mirrors the change.
        if let Some(bus) = self.app_handle.try_state::<crate::control::ControlStateBus>() {
            bus.self_state();
        }
    }
}
