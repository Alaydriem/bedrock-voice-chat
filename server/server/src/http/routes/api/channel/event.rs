use crate::http::openapi::CustomJsonResponse;
use crate::services::{ChannelMembershipService, MetricsService};
use crate::stream::quic::{CacheManager, WebhookReceiver};
use common::structs::{
    channel::{
        ChannelEvent,
        ChannelEvents::{Delete, Join, Leave},
    },
    packet::{
        ChannelEventPacket, PacketOwner, PacketType, QuicNetworkPacket, QuicNetworkPacketData,
    },
};
use rocket::{State, http::Status, mtls::Certificate, serde::json::Json};
use rocket_okapi::openapi;
use std::sync::Arc;

/// Update a channel's membership.
#[openapi(tag = "Channels")]
#[put("/<id>", data = "<event>")]
pub async fn channel_event(
    identity: Certificate<'_>,
    cache_manager: &State<CacheManager>,
    id: &str,
    webhook_receiver: &State<WebhookReceiver>,
    metrics: &State<Arc<MetricsService>>,
    event: Json<ChannelEvent>,
) -> CustomJsonResponse<bool> {
    let user = match identity.subject().common_name() {
        Some(user) => user.to_string(),
        None => {
            return CustomJsonResponse::error(Status::Forbidden);
        }
    };

    let event = event.0;
    let channel_collection = cache_manager.get_channel_collection();

    // Existence guard (unchanged): a Delete on a missing channel still fans a
    // Delete; any other event on a missing channel is a bad request.
    if channel_collection.get(id).await.is_none() {
        if event.event.eq(&Delete) {
            send_channel_event(channel_packet(Delete, user, id), webhook_receiver).await;
            return CustomJsonResponse::ok(true);
        }
        return CustomJsonResponse::custom(Status::BadRequest, Some(false));
    }

    match event.event {
        Join => {
            ChannelMembershipService::join(&channel_collection, webhook_receiver.inner(), user, id)
                .await;
            metrics.record_channel_join();
        }
        Leave => {
            ChannelMembershipService::leave(
                &channel_collection,
                webhook_receiver.inner(),
                user,
                id,
                false,
            )
            .await;
            metrics.record_channel_leave();
        }
        other => {
            send_channel_event(channel_packet(other, user, id), webhook_receiver).await;
        }
    }

    CustomJsonResponse::ok(true)
}

fn channel_packet(
    event: common::structs::channel::ChannelEvents,
    user: String,
    id: &str,
) -> QuicNetworkPacket {
    QuicNetworkPacket {
        owner: Some(PacketOwner {
            name: String::from("channel_api"),
            client_id: vec![0u8; 0],
        }),
        packet_type: PacketType::ChannelEvent,
        data: QuicNetworkPacketData::ChannelEvent(ChannelEventPacket::new(
            event,
            user,
            id.to_string(),
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
