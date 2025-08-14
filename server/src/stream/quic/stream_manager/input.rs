use common::traits::StreamTrait;
use common::structs::packet::{QuicNetworkPacket, PlayerPresenceEvent, ConnectionEventType, PacketOwner, PacketType, QuicNetworkPacketData};
use anyhow::Error;
use s2n_quic::stream::ReceiveStream;
use tokio::sync::mpsc;
use crate::stream::quic::{ServerInputPacket, WebhookReceiver};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Helper function to create a short hash representation of client_id
fn client_id_hash(client_id: &[u8]) -> String {
    let mut hasher = DefaultHasher::new();
    client_id.hash(&mut hasher);
    format!("{:x}", hasher.finish() & 0xFFFF) // Take only last 4 hex digits for readability
}

pub(crate) struct InputStream {
    receiver: Option<ReceiveStream>,
    // Producer to send received data to other components
    producer: Option<mpsc::UnboundedSender<ServerInputPacket>>,
    is_stopped: Arc<AtomicBool>,
    // Player identity from first packet with owner
    player_id: Option<String>,
    client_id: Option<Vec<u8>>,
    // Callback to notify when disconnect happens (for cache cleanup)
    // Parameters: (player_name, client_id)
    disconnect_callback: Option<Box<dyn Fn(String, Vec<u8>) + Send + Sync>>,
    // Webhook receiver for sending presence events
    webhook_receiver: Option<WebhookReceiver>,
}

impl InputStream {
    pub fn new(
        receiver: Option<ReceiveStream>,
        producer: Option<mpsc::UnboundedSender<ServerInputPacket>>,
    ) -> Self {
        Self {
            receiver,
            producer,
            is_stopped: Arc::new(AtomicBool::new(true)),
            player_id: None,
            client_id: None,
            disconnect_callback: None,
            webhook_receiver: None,
        }
    }

    pub fn set_producer(&mut self, producer: mpsc::UnboundedSender<ServerInputPacket>) {
        self.producer = Some(producer);
    }

    #[allow(unused)]
    pub fn set_receiver(&mut self, receiver: ReceiveStream) {
        self.receiver = Some(receiver);
    }

    pub fn set_disconnect_callback(&mut self, callback: Box<dyn Fn(String, Vec<u8>) + Send + Sync>) {
        self.disconnect_callback = Some(callback);
    }

    pub fn set_webhook_receiver(&mut self, webhook_receiver: WebhookReceiver) {
        self.webhook_receiver = Some(webhook_receiver);
    }

    #[allow(unused)]
    pub fn get_player_id(&self) -> Option<&String> {
        self.player_id.as_ref()
    }
}

impl StreamTrait for InputStream {
    fn is_stopped(&self) -> bool {
        self.is_stopped.load(Ordering::Relaxed)
    }

    async fn stop(&mut self) -> Result<(), Error> {
        tracing::info!("Stopping QUIC input stream");
        self.is_stopped.store(true, Ordering::Relaxed);
        Ok(())
    }

    async fn start(&mut self) -> Result<(), Error> {
        tracing::info!("Starting QUIC input stream");
        self.is_stopped.store(false, Ordering::Relaxed);
        
        if let (Some(mut receiver), Some(producer)) = (self.receiver.take(), self.producer.clone()) {
            let mut packet_buffer = Vec::<u8>::new();
            
            // Handle incoming packets from this connection
            loop {
                match receiver.receive().await {
                    Ok(Some(data)) => {
                        packet_buffer.append(&mut data.to_vec());
                        
                        match QuicNetworkPacket::from_stream(&mut packet_buffer) {
                            Ok(packets) => {
                                for packet in packets {
                                    // Extract player identity from first packet with owner
                                    if self.player_id.is_none() && packet.owner.is_some() {
                                        let owner = packet.owner.as_ref().unwrap();
                                        self.player_id = Some(owner.name.clone());
                                        self.client_id = Some(owner.client_id.clone());
                                        let client_hash = client_id_hash(&owner.client_id);
                                        tracing::info!("Initialized player identity: {} (client: {})", owner.name, client_hash);
                                        
                                        // Send Connected presence event in separate task (non-blocking)
                                        if let Some(webhook_receiver) = &self.webhook_receiver {
                                            let player_name = owner.name.clone();
                                            let webhook_receiver_clone = webhook_receiver.clone();
                                            tokio::spawn(async move {
                                                let timestamp = std::time::SystemTime::now()
                                                    .duration_since(std::time::UNIX_EPOCH)
                                                    .unwrap()
                                                    .as_millis() as i64;
                                                    
                                                let presence_packet = QuicNetworkPacket {
                                                    owner: Some(PacketOwner {
                                                        name: String::from("api"),
                                                        client_id: vec![0u8; 0],
                                                    }),
                                                    packet_type: PacketType::PlayerPresence,
                                                    data: QuicNetworkPacketData::PlayerPresence(PlayerPresenceEvent {
                                                        player_name: player_name.clone(),
                                                        timestamp,
                                                        event_type: ConnectionEventType::Connected,
                                                    }),
                                                };
                                                
                                                if let Err(e) = webhook_receiver_clone.send_packet(presence_packet).await {
                                                    tracing::error!("Failed to send player connected event: {}", e);
                                                }

                                                tracing::debug!("Broadcast player connected event {}", player_name);
                                            });
                                        }
                                    }
                                    
                                    let server_packet = ServerInputPacket { data: packet };
                                    if let Err(e) = producer.send(server_packet) {
                                        tracing::error!("Failed to send packet to producer: {}", e);
                                        break;
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::error!("Failed to parse QUIC network packet: {}", e);
                                continue;
                            }
                        }
                    }
                    Ok(None) => {
                        // Stream ended - handle disconnect
                        tracing::info!("QUIC receive stream ended");
                        
                        // Send disconnect signal using QUIC error code 204
                        if let Err(e) = receiver.stop_sending((204u8).into()) {
                            tracing::warn!("Failed to send QUIC disconnect signal: {}", e);
                        }
                        
                        // Call disconnect callback for cache cleanup
                        if let (Some(callback), Some(player_id), Some(client_id)) = (&self.disconnect_callback, &self.player_id, &self.client_id) {
                            callback(player_id.clone(), client_id.clone());
                            
                            // Send Disconnected presence event in separate task (non-blocking)
                            if let Some(webhook_receiver) = &self.webhook_receiver {
                                let player_name = player_id.clone();
                                let webhook_receiver_clone = webhook_receiver.clone();
                                tokio::spawn(async move {
                                    let timestamp = std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap()
                                        .as_millis() as i64;
                                        
                                    let presence_packet = QuicNetworkPacket {
                                        owner: Some(PacketOwner {
                                            name: String::from("api"),
                                            client_id: vec![0u8; 0],
                                        }),
                                        packet_type: PacketType::PlayerPresence,
                                        data: QuicNetworkPacketData::PlayerPresence(PlayerPresenceEvent {
                                            player_name: player_name.clone(),
                                            timestamp,
                                            event_type: ConnectionEventType::Disconnected,
                                        }),
                                    };
                                    
                                    if let Err(e) = webhook_receiver_clone.send_packet(presence_packet).await {
                                        tracing::error!("Failed to send player disconnected event: {}", e);
                                    }

                                    tracing::debug!("Broadcast player disconnected event {}", player_name);
                                });
                            }
                        }
                        
                        break;
                    }
                    Err(e) => {
                        tracing::error!("Error receiving from QUIC stream: {}", e);
                        
                        // Also send disconnect signal on error
                        if let Err(stop_err) = receiver.stop_sending((204u8).into()) {
                            tracing::warn!("Failed to send QUIC disconnect signal on error: {}", stop_err);
                        }
                        
                        // Call disconnect callback for cache cleanup
                        if let (Some(callback), Some(player_id), Some(client_id)) = (&self.disconnect_callback, &self.player_id, &self.client_id) {
                            callback(player_id.clone(), client_id.clone());
                            
                            // Send Disconnected presence event in separate task (non-blocking)
                            if let Some(webhook_receiver) = &self.webhook_receiver {
                                let player_name = player_id.clone();
                                let webhook_receiver_clone = webhook_receiver.clone();
                                tokio::spawn(async move {
                                    let timestamp = std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap()
                                        .as_millis() as i64;
                                        
                                    let presence_packet = QuicNetworkPacket {
                                        owner: Some(PacketOwner {
                                            name: String::from("api"),
                                            client_id: vec![0u8; 0],
                                        }),
                                        packet_type: PacketType::PlayerPresence,
                                        data: QuicNetworkPacketData::PlayerPresence(PlayerPresenceEvent {
                                            player_name,
                                            timestamp,
                                            event_type: ConnectionEventType::Disconnected,
                                        }),
                                    };
                                    
                                    if let Err(e) = webhook_receiver_clone.send_packet(presence_packet).await {
                                        tracing::error!("Failed to send player disconnected event: {}", e);
                                    }
                                });
                            }
                        }
                        
                        break;
                    }
                }
            }
        }
        
        self.is_stopped.store(true, Ordering::Relaxed);
        Ok(())
    }

    async fn metadata(&mut self, key: String, value: String) -> Result<(), Error> {
        tracing::info!("Setting metadata for QUIC input stream: {} = {}", key, value);
        Ok(())
    }
}