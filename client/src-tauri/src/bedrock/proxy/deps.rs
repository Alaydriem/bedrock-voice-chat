use std::sync::Arc;

use super::connect_error_channel::BedrockConnectErrorChannel;
use super::event_emitter::BedrockEventEmitter;
use super::jukebox::{JukeboxBeaconCache, JukeboxEjectInjector};
use super::player_state_cache::BedrockPlayerStateCache;
use super::presence::{AnnounceInjector, PresenceInjector, QueryStateInjector};
use crate::bedrock::ProtocolGatingService;
use crate::control::{ControlActionSender, ControlStateBus};

// Shared service dependencies every BedrockProxyManager needs regardless of
// backend. Required at construction so the Direct and Realm connect paths
// cannot diverge on wiring.
pub(crate) struct ProxyDeps {
    pub(crate) player_state_cache: Arc<BedrockPlayerStateCache>,
    pub(crate) gating: Arc<ProtocolGatingService>,
    pub(crate) beacon_cache: Arc<JukeboxBeaconCache>,
    pub(crate) error_channel: Arc<BedrockConnectErrorChannel>,
    pub(crate) event_emitter: Arc<BedrockEventEmitter>,
    pub(crate) eject_injector: Arc<JukeboxEjectInjector>,
    pub(crate) presence_injector: Arc<PresenceInjector>,
    pub(crate) announce_injector: Arc<AnnounceInjector>,
    pub(crate) control_tx: ControlActionSender,
    pub(crate) query_state_injector: Arc<QueryStateInjector>,
    pub(crate) state_bus: ControlStateBus,
}

impl ProxyDeps {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        player_state_cache: Arc<BedrockPlayerStateCache>,
        gating: Arc<ProtocolGatingService>,
        beacon_cache: Arc<JukeboxBeaconCache>,
        error_channel: Arc<BedrockConnectErrorChannel>,
        event_emitter: Arc<BedrockEventEmitter>,
        eject_injector: Arc<JukeboxEjectInjector>,
        presence_injector: Arc<PresenceInjector>,
        announce_injector: Arc<AnnounceInjector>,
        control_tx: ControlActionSender,
        query_state_injector: Arc<QueryStateInjector>,
        state_bus: ControlStateBus,
    ) -> Self {
        Self {
            player_state_cache,
            gating,
            beacon_cache,
            error_channel,
            event_emitter,
            eject_injector,
            presence_injector,
            announce_injector,
            control_tx,
            query_state_injector,
            state_bus,
        }
    }
}
