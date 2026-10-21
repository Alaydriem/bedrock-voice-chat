use std::sync::{Arc, Mutex};

use tokio::sync::{mpsc, oneshot};

use super::{ProtectedGroupCommand, SealGate};

/// The app's one active protected group: the gate the audio threads use, the signal that stops
/// the group's runner, and the channel it routes through. A client is in at most one group, so a
/// new group replaces the old.
pub struct ProtectedGroupSlot {
    gate: Arc<SealGate>,
    active: Mutex<Option<(oneshot::Sender<()>, String)>>,
    // The protected group's channel, remembered past the runner's end: a lost group leaves the
    // gate protected until this client is out of that channel, however it gets out.
    entered: Mutex<Option<String>>,
    commands: Mutex<Option<mpsc::Sender<ProtectedGroupCommand>>>,
}

impl ProtectedGroupSlot {
    pub fn new() -> Self {
        Self {
            gate: Arc::new(SealGate::new()),
            active: Mutex::new(None),
            entered: Mutex::new(None),
            commands: Mutex::new(None),
        }
    }

    pub fn new_shared() -> Arc<Self> {
        Arc::new(Self::new())
    }

    pub fn gate(&self) -> Arc<SealGate> {
        self.gate.clone()
    }

    /// Registers a new runner, its channel and the queue its decisions go to, stopping any runner
    /// already registered.
    pub fn install(
        &self,
        stop: oneshot::Sender<()>,
        channel_id: String,
        commands: mpsc::Sender<ProtectedGroupCommand>,
    ) {
        if let Ok(mut entered) = self.entered.lock() {
            *entered = Some(channel_id.clone());
        }
        if let Ok(mut slot) = self.commands.lock() {
            *slot = Some(commands);
        }
        if let Ok(mut slot) = self.active.lock() {
            if let Some((previous, _)) = slot.replace((stop, channel_id)) {
                let _ = previous.send(());
            }
        }
    }

    /// The server reports this client out of `channel_id` — by any path, the in-game panel or a
    /// controller included. If that is the protected group's channel, stop the group and let
    /// audio flow again: the server already routes this client elsewhere, so nothing unsealed can
    /// reach the old group. Returns whether it was.
    pub fn release_if_left(&self, channel_id: &str) -> bool {
        let was_ours = self
            .entered
            .lock()
            .map(|mut entered| {
                let ours = entered.as_deref() == Some(channel_id);
                if ours {
                    *entered = None;
                }
                ours
            })
            .unwrap_or(false);
        if was_ours {
            self.stop();
            self.gate.set_protected(false);
        }
        was_ours
    }

    /// Stops the active runner, if any, and drops every key. The gate's protection is left as it
    /// is: lifting it is a separate decision, made once the channel has been left.
    pub fn stop(&self) -> bool {
        let stopped = self
            .active
            .lock()
            .ok()
            .and_then(|mut slot| slot.take())
            .map(|(stop, _)| stop.send(()).is_ok())
            .unwrap_or(false);
        self.gate.clear();
        stopped
    }

    /// Where a person's admission decisions go, while a runner is alive.
    pub fn commands(&self) -> Option<mpsc::Sender<ProtectedGroupCommand>> {
        self.commands
            .lock()
            .ok()
            .and_then(|slot| slot.clone())
            .filter(|commands| !commands.is_closed())
    }

    /// The channel of the group this client holds, while its runner is alive.
    pub fn channel(&self) -> Option<String> {
        self.active.lock().ok().and_then(|slot| {
            slot.as_ref()
                .filter(|(stop, _)| !stop.is_closed())
                .map(|(_, channel)| channel.clone())
        })
    }

    pub fn is_active(&self) -> bool {
        self.channel().is_some()
    }
}

impl Default for ProtectedGroupSlot {
    fn default() -> Self {
        Self::new()
    }
}
