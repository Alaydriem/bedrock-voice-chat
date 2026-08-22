use crate::http::openapi::CustomJsonResponse;
use crate::services::{ChannelMembershipService, MetricsService};
use crate::stream::quic::{CacheManager, WebhookReceiver};
use common::structs::{
    channel::{
        ChannelEvent,
        ChannelEvents::{Create, Delete, Join, Leave, Rename},
    },
    packet::{
        ChannelEventPacket, PacketSender, PacketType, QuicNetworkPacket, QuicNetworkPacketData,
    },
};
use crate::http::guards::PlayerGuard;
use rocket::{State, http::Status, serde::json::Json};
use rocket_okapi::openapi;
use std::sync::Arc;

/// Update a channel's membership.
#[openapi(tag = "Channels")]
#[put("/<id>", data = "<event>")]
pub async fn channel_event(
    guard: PlayerGuard,
    cache_manager: &State<CacheManager>,
    id: &str,
    webhook_receiver: &State<WebhookReceiver>,
    metrics: &State<Arc<MetricsService>>,
    event: Json<ChannelEvent>,
) -> CustomJsonResponse<bool> {
    // Derived from the resolved player rather than read off the certificate CN, so a
    // legacy bare-gamertag certificate produces the same canonical identity as a current one.
    let Some(gamertag) = guard.player.gamertag.clone() else {
        return CustomJsonResponse::error(Status::Forbidden);
    };
    let user = guard.player.game.membership_key(&gamertag);

    let event = event.0;
    let channel_collection = cache_manager.get_channel_collection();

    // A Delete for a channel that does not exist used to fan a Delete anyway, which every
    // connected client then processed for an id that was never real.
    let Some(channel) = channel_collection.get(id).await else {
        return CustomJsonResponse::custom(Status::NotFound, Some(false));
    };

    // Explicit rather than a catch-all arm. A future `ChannelEvents` variant must be given a
    // disposition instead of silently inheriting an unguarded fan-out, which is how Delete
    // ended up bypassing the creator check that `DELETE /<id>` enforces.
    match event.event {
        // Open by design: the channel id is a share code, so being handed one is the whole
        // mechanism for joining a group.
        Join => {
            ChannelMembershipService::join(
                &channel_collection,
                webhook_receiver.inner(),
                &user,
                id,
            )
            .await;
            metrics.record_channel_join();
        }
        Leave => {
            ChannelMembershipService::leave(
                &channel_collection,
                webhook_receiver.inner(),
                &user,
                id,
                false,
            )
            .await;
            metrics.record_channel_leave();
        }
        Delete => {
            if !channel.creator.eq(&user) {
                return CustomJsonResponse::custom(Status::Unauthorized, Some(false));
            }
            // Removed as well as fanned, so this route and `DELETE /<id>` cannot leave the
            // server and the clients disagreeing about whether the channel exists.
            channel_collection.remove(id).await;
            send_channel_event(channel_packet(Delete, user, id, &channel), webhook_receiver).await;
        }
        Rename => {
            if !channel.creator.eq(&user) {
                return CustomJsonResponse::custom(Status::Unauthorized, Some(false));
            }
            send_channel_event(channel_packet(Rename, user, id, &channel), webhook_receiver).await;
        }
        // Creation is `POST /api/channel`. Accepting it here would fan a Create for a channel
        // that already exists, so it is refused rather than ignored.
        Create => {
            return CustomJsonResponse::custom(Status::BadRequest, Some(false));
        }
    }

    CustomJsonResponse::ok(true)
}

fn channel_packet(
    event: common::structs::channel::ChannelEvents,
    user: common::PlayerIdentity,
    id: &str,
    channel: &common::structs::channel::Channel,
) -> QuicNetworkPacket {
    QuicNetworkPacket {
        sender: Some(PacketSender::for_service(PacketSender::CHANNEL_API)),
        packet_type: PacketType::ChannelEvent,
        data: QuicNetworkPacketData::ChannelEvent(ChannelEventPacket::new(
            event,
            user,
            id.to_string(),
            Some(channel.name.clone()),
            Some(channel.creator.clone()),
        )),
        // Not a server fan-out to one connection, so this envelope carries no sequence.
        ..Default::default()
    }
}

async fn send_channel_event(packet: QuicNetworkPacket, webhook_receiver: &State<WebhookReceiver>) {
    if let Err(e) = webhook_receiver.send_packet(packet).await {
        tracing::error!("Failed to send packet to QUIC server: {}", e);
    }
}
