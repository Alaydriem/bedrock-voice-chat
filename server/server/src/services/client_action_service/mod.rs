use common::structs::channel::ChannelCollection;
use common::structs::control::{
    ClientAction, ClientActionType, PlayerPreference, PreferenceKey, QueryState,
};
use common::structs::packet::{
    ClientActionPacket, PacketDirection, PacketType, QuicNetworkPacket, QuicNetworkPacketData,
};

use crate::services::ChannelMembershipService;
use crate::stream::quic::WebhookReceiver;
use crate::stream::quic::connection_registry::ConnectionRegistry;
use crate::stream::quic::{CacheTrait, PlayerPreferenceCache, PlayerStateCache};

/// Applies inbound `ClientAction`s. Self/preference actions are delivered back to
/// the authenticated actor's own connection as a ClientBound `ClientAction`; group
/// actions go through `ChannelMembershipService`. The wire `action.id` is never
/// trusted for routing — the authenticated actor the caller supplies is
/// authoritative.
pub struct ClientActionService {
    recording_enabled: bool,
}

impl ClientActionService {
    pub fn new(recording_enabled: bool) -> Self {
        Self { recording_enabled }
    }

    /// Whether this action may be applied at all.
    ///
    /// Only arming a recording is ever refused. Stopping stays available on a server
    /// whose operator turned recording off while someone was already recording, and
    /// nothing else this service routes is subject to operator policy.
    ///
    /// The recording itself is written on the player's own machine, so a refusal here
    /// states the server's answer rather than preventing anything.
    pub fn permits(&self, action: &ClientActionType) -> bool {
        !matches!(action, ClientActionType::SetRecording(true)) || self.recording_enabled
    }

    /// Delivers a self/preference action to the authenticated actor's own connection.
    ///
    /// `actor_identity` is the canonical `game:gamertag` and is authoritative; the wire
    /// `action.id` is overwritten with it. A bare gamertag here matches no connection, so
    /// callers holding one compose it with `ClientAction::actor_key` first.
    ///
    /// Returns whether a live connection received it.
    pub fn route_self(
        &self,
        action: &ClientAction,
        actor_identity: &str,
        registry: &ConnectionRegistry,
    ) -> bool {
        let packet = ClientActionPacket::new(
            ClientAction {
                id: actor_identity.to_string(),
                game: action.game.clone(),
                action: action.action.clone(),
            },
            PacketDirection::ClientBound,
        );
        let envelope = QuicNetworkPacket {
            packet_type: PacketType::ClientAction,
            data: QuicNetworkPacketData::ClientAction(packet),
                    // Not a server fan-out, so this envelope carries no sequence.
            ..Default::default()
        };
        registry.send_to_player(actor_identity, &envelope)
    }

    /// `route_self` plus an optimistic cache echo: when a live connection actually
    /// received the action, the same change is folded into the control-plane caches
    /// the panel polls. The client's confirming report is debounced (~200ms), so
    /// without the echo a poll racing that round trip serves pre-action state and
    /// visibly flaps the panel's controls. Self-state is only patched when the
    /// client has already reported once (never fabricates a full `QueryState` from
    /// one field); preferences upsert, since the action carries the whole entry.
    pub async fn route_self_with_echo(
        &self,
        action: &ClientAction,
        actor_identity: &str,
        registry: &ConnectionRegistry,
        player_state: &PlayerStateCache,
        preferences: &PlayerPreferenceCache,
    ) -> bool {
        if !self.route_self(action, actor_identity, registry) {
            return false;
        }

        match &action.action {
            ClientActionType::SetMuted(on) => {
                Self::patch_self_state(player_state, actor_identity, |s| s.muted = *on).await;
            }
            ClientActionType::SetDeafened(on) => {
                Self::patch_self_state(player_state, actor_identity, |s| s.deafened = *on).await;
            }
            ClientActionType::SetRecording(on) => {
                Self::patch_self_state(player_state, actor_identity, |s| s.recording = *on).await;
            }
            ClientActionType::SetVolume { target, volume } => {
                // Mirror the client's own sanitation (ignore non-finite, clamp to
                // [0,1]) so the echo never publishes a gain the client won't apply.
                if volume.is_finite() {
                    let volume = volume.clamp(0.0, 1.0);
                    Self::patch_preference(preferences, actor_identity, target, |p| {
                        p.volume = volume
                    })
                    .await;
                }
            }
            ClientActionType::SetHeard { target, muted } => {
                Self::patch_preference(preferences, actor_identity, target, |p| p.muted = *muted)
                    .await;
            }
            _ => {}
        }
        true
    }

    async fn patch_self_state(
        player_state: &PlayerStateCache,
        actor_identity: &str,
        apply: impl FnOnce(&mut QueryState),
    ) {
        if let Some(mut state) = player_state.get(&actor_identity.to_string()).await {
            apply(&mut state);
            player_state.set(actor_identity.to_string(), state).await;
        }
    }

    async fn patch_preference(
        preferences: &PlayerPreferenceCache,
        actor_identity: &str,
        target: &str,
        apply: impl FnOnce(&mut PlayerPreference),
    ) {
        let key = PreferenceKey::new(actor_identity, target);
        let mut pref = preferences
            .get(&key)
            .await
            .unwrap_or_else(|| PlayerPreference {
                owner: actor_identity.to_string(),
                target: target.to_string(),
                volume: 1.0,
                muted: false,
            });
        apply(&mut pref);
        preferences.set(key, pref).await;
    }

    /// Applies a group action for the authenticated actor (cert-CN form
    /// `game:gamertag`). `CreateGroup` and `JoinGroup` are MOVES — the actor's
    /// current groups are left first, so a player occupies at most one group
    /// through this plane (multi-membership renders as an invalid state in the
    /// desktop client, whose own flow also moves). Returns the new nanoid for
    /// `CreateGroup`; errors when a `JoinGroup` targets a channel that does not
    /// exist (never creates phantom membership, never disturbs the current
    /// group on a bad code). Any channel left empty is closed.
    pub async fn route_group(
        action: &ClientActionType,
        actor_cn: &str,
        channels: &ChannelCollection,
        webhook: &WebhookReceiver,
    ) -> anyhow::Result<Option<String>> {
        match action {
            ClientActionType::CreateGroup => {
                Self::leave_all(channels, webhook, actor_cn).await;
                let id = ChannelMembershipService::create(
                    channels,
                    webhook,
                    format!("{actor_cn} group"),
                    actor_cn.to_string(),
                )
                .await;
                ChannelMembershipService::join(channels, webhook, actor_cn.to_string(), &id).await;
                Ok(Some(id))
            }
            ClientActionType::JoinGroup { channel } => {
                // Validate the code BEFORE leaving anything: a typo must not
                // kick the actor out of their current group.
                if channels.get(channel).await.is_none() {
                    anyhow::bail!("channel does not exist: {channel}");
                }
                // A repeat join of the current group is a no-op, not a move —
                // leaving first would close the group under its last member.
                if channels
                    .get_player_channels(actor_cn)
                    .iter()
                    .any(|c| c == channel)
                {
                    return Ok(None);
                }
                Self::leave_all(channels, webhook, actor_cn).await;
                if ChannelMembershipService::join(channels, webhook, actor_cn.to_string(), channel)
                    .await
                {
                    Ok(None)
                } else {
                    anyhow::bail!("channel does not exist: {channel}")
                }
            }
            ClientActionType::LeaveGroup => {
                Self::leave_all(channels, webhook, actor_cn).await;
                Ok(None)
            }
            _ => Ok(None),
        }
    }

    async fn leave_all(channels: &ChannelCollection, webhook: &WebhookReceiver, actor_cn: &str) {
        for cid in channels.get_player_channels(actor_cn) {
            ChannelMembershipService::leave(channels, webhook, actor_cn.to_string(), &cid, true)
                .await;
        }
    }
}

