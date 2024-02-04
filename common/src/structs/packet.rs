use crate::Coordinate;
use anyhow::anyhow;
use serde::{ Deserialize, Serialize };

use super::channel::ChannelEvents;

/// The packet type
#[derive(Debug, Clone, Deserialize, Serialize, Eq, PartialEq)]
pub enum PacketType {
    AudioFrame,
    PlayerData,
    ChannelEvent,
    Debug,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct QuicNetworkPacketCollection {
    pub frames: Vec<QuicNetworkPacket>,
    pub positions: PlayerDataPacket,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum QuicNetworkPacketData {
    AudioFrame(AudioFramePacket),
    PlayerData(PlayerDataPacket),
    ChannelEvent(ChannelEventPacket),
    Debug(DebugPacket),
}

/// A network packet to be sent via QUIC
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct QuicNetworkPacket {
    pub packet_type: PacketType,
    pub author: String,
    pub client_id: Vec<u8>,
    pub data: QuicNetworkPacketData,
}

/// Magic header
pub const QUICK_NETWORK_PACKET_HEADER: &[u8; 5] = &[251, 33, 51, 0, 27];

impl QuicNetworkPacketCollection {
    /// write_all() returns a stream of data that isn't cleanly deliniated by packets
    /// And flush() may be called at any time.
    /// This takes a reference to an existing Vec<u8>, from a receive_stream() and
    /// returns all the QuicNetworkPackets that were sent.
    /// The packet is mutated in place with any partial data
    pub fn from_stream(
        packet: &mut Vec<u8>
    ) -> Result<Vec<QuicNetworkPacketCollection>, anyhow::Error> {
        let mut packets = Vec::<QuicNetworkPacketCollection>::new();

        // If we didn't get anything we can return immediately
        if packet.len() == 0 {
            return Ok(packets);
        }

        loop {
            // The first 5 bytes of the packet should always be the magic header we use to indicate a new packet has started
            // If these bytes don't match the magic header, then rip them off the packet and try again
            // If we don't get any bytes from this then the packet is malformed, and we need more data
            match packet.get(0..5) {
                Some(header) =>
                    match header.to_vec().eq(&QUICK_NETWORK_PACKET_HEADER) {
                        true => {}
                        false => {
                            // If the first 5 bytes exist, but they aren't the magic packet header, then we've lost packets
                            // To prevent packet loss from causing a memory leak, we need to advance the pointer in the packet to the position of the next instance of the magic header.

                            match
                                packet
                                    .windows(QUICK_NETWORK_PACKET_HEADER.len())
                                    .position(|window| window == QUICK_NETWORK_PACKET_HEADER)
                            {
                                Some(position) => {
                                    // Reset the packet to the starting point of the magic packet header
                                    *packet = packet
                                        .get(position..packet.len())
                                        .unwrap()
                                        .to_vec();

                                    packet.shrink_to(packet.len());
                                    packet.truncate(packet.len());

                                    // Try to continue
                                    continue;
                                }
                                None => {
                                    // If this happens we have a bunch of random data without a magic packet.
                                    // We should reset the buffer because we can't do anything with this
                                    *packet = Vec::new();

                                    packet.shrink_to(packet.len());
                                    packet.truncate(packet.len());
                                    break;
                                }
                            }
                        }
                    }
                None => {
                    break;
                }
            }

            // The next 8 bytes should be the packet length
            let length = match packet.get(5..13) {
                Some(bytes) => usize::from_be_bytes(bytes.try_into().unwrap()),
                None => {
                    break;
                }
            };

            if packet.len() >= length + 13 {
                let packet_to_process = packet
                    .get(0..length + 13)
                    .unwrap()
                    .to_vec();

                *packet = packet
                    .get(13 + length..packet.len())
                    .unwrap()
                    .to_vec();

                packet.shrink_to(packet.len());
                packet.truncate(packet.len());

                match Self::from_vec(&packet_to_process[13..]) {
                    Ok(p) => packets.push(p),
                    Err(_) => {
                        continue;
                    }
                };
            } else {
                break;
            }
        }

        return Ok(packets);
    }

    /// Converts the packet into a parseable string
    pub fn to_vec(&self) -> Result<Vec<u8>, anyhow::Error> {
        match ron::to_string(&self) {
            Ok(rs) => {
                let mut header: Vec<u8> = QUICK_NETWORK_PACKET_HEADER.to_vec();
                let mut len = rs.as_bytes().len().to_be_bytes().to_vec();
                let mut data = rs.as_bytes().to_vec();

                header.append(&mut len);
                header.append(&mut data);

                return Ok(header);
            }
            Err(e) => {
                return Err(anyhow!("Could not parse packet. {}", e.to_string()));
            }
        }
    }

    /// Convers a vec back into a raw packet
    pub fn from_vec(data: &[u8]) -> Result<Self, anyhow::Error> {
        match std::str::from_utf8(data) {
            Ok(ds) =>
                match ron::from_str::<QuicNetworkPacketCollection>(&ds) {
                    Ok(packet) => {
                        return Ok(packet);
                    }
                    Err(e) => {
                        return Err(anyhow!("{}", e.to_string()));
                    }
                }
            Err(e) => {
                return Err(
                    anyhow!(
                        "Unable to deserialize RON packet. Possible packet length issue? {}",
                        e.to_string()
                    )
                );
            }
        }
    }
}

impl QuicNetworkPacket {
    /// write_all() returns a stream of data that isn't cleanly deliniated by packets
    /// And flush() may be called at any time.
    /// This takes a reference to an existing Vec<u8>, from a receive_stream() and
    /// returns all the QuicNetworkPackets that were sent.
    /// The packet is mutated in place with any partial data
    pub fn from_stream(packet: &mut Vec<u8>) -> Result<Vec<QuicNetworkPacket>, anyhow::Error> {
        let mut packets = Vec::<QuicNetworkPacket>::new();

        // If we didn't get anything we can return immediately
        if packet.len() == 0 {
            return Ok(packets);
        }

        loop {
            // The first 5 bytes of the packet should always be the magic header we use to indicate a new packet has started
            // If these bytes don't match the magic header, then rip them off the packet and try again
            // If we don't get any bytes from this then the packet is malformed, and we need more data
            match packet.get(0..5) {
                Some(header) =>
                    match header.to_vec().eq(&QUICK_NETWORK_PACKET_HEADER) {
                        true => {}
                        false => {
                            // If the first 5 bytes exist, but they aren't the magic packet header, then we've lost packets
                            // To prevent packet loss from causing a memory leak, we need to advance the pointer in the packet to the position of the next instance of the magic header.

                            match
                                packet
                                    .windows(QUICK_NETWORK_PACKET_HEADER.len())
                                    .position(|window| window == QUICK_NETWORK_PACKET_HEADER)
                            {
                                Some(position) => {
                                    // Reset the packet to the starting point of the magic packet header
                                    *packet = packet
                                        .get(position..packet.len())
                                        .unwrap()
                                        .to_vec();

                                    packet.shrink_to(packet.len());
                                    packet.truncate(packet.len());

                                    // Try to continue
                                    continue;
                                }
                                None => {
                                    // If this happens we have a bunch of random data without a magic packet.
                                    // We should reset the buffer because we can't do anything with this
                                    *packet = Vec::new();

                                    packet.shrink_to(packet.len());
                                    packet.truncate(packet.len());
                                    break;
                                }
                            }
                        }
                    }
                None => {
                    break;
                }
            }

            // The next 8 bytes should be the packet length
            let length = match packet.get(5..13) {
                Some(bytes) => usize::from_be_bytes(bytes.try_into().unwrap()),
                None => {
                    break;
                }
            };

            if packet.len() >= length + 13 {
                let packet_to_process = packet
                    .get(0..length + 13)
                    .unwrap()
                    .to_vec();

                *packet = packet
                    .get(13 + length..packet.len())
                    .unwrap()
                    .to_vec();

                packet.shrink_to(packet.len());
                packet.truncate(packet.len());

                match Self::from_vec(&packet_to_process[13..]) {
                    Ok(p) => packets.push(p),
                    Err(_) => {
                        continue;
                    }
                };
            } else {
                break;
            }
        }

        return Ok(packets);
    }

    /// Converts the packet into a parseable string
    pub fn to_vec(&self) -> Result<Vec<u8>, anyhow::Error> {
        match ron::to_string(&self) {
            Ok(rs) => {
                let mut header: Vec<u8> = QUICK_NETWORK_PACKET_HEADER.to_vec();
                let mut len = rs.as_bytes().len().to_be_bytes().to_vec();
                let mut data = rs.as_bytes().to_vec();

                header.append(&mut len);
                header.append(&mut data);

                return Ok(header);
            }
            Err(e) => {
                return Err(anyhow!("Could not parse packet. {}", e.to_string()));
            }
        }
    }

    /// Convers a vec back into a raw packet
    pub fn from_vec(data: &[u8]) -> Result<Self, anyhow::Error> {
        match std::str::from_utf8(data) {
            Ok(ds) =>
                match ron::from_str::<QuicNetworkPacket>(&ds) {
                    Ok(packet) => {
                        return Ok(packet);
                    }
                    Err(e) => {
                        return Err(anyhow!("{}", e.to_string()));
                    }
                }
            Err(e) => {
                return Err(
                    anyhow!(
                        "Unable to deserialize RON packet. Possible packet length issue? {}",
                        e.to_string()
                    )
                );
            }
        }
    }

    /// Whether or not a packet should be broadcasted
    pub fn broadcast(&self) -> bool {
        match self.packet_type {
            PacketType::AudioFrame => false,
            PacketType::Debug => true,
            PacketType::PlayerData => true,
            PacketType::ChannelEvent => true,
        }
    }

    /// Returns the underlying data frame.
    pub fn get_data(&self) -> Option<&QuicNetworkPacketData> {
        Some(&self.data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deconstruct() {
        let packet = QuicNetworkPacket {
            packet_type: PacketType::AudioFrame,
            author: "User".to_string(),
            client_id: vec![24; 0],
            data: QuicNetworkPacketData::AudioFrame(AudioFramePacket {
                length: 240,
                sample_rate: 48000,
                data: vec![240; 0],
                author: "User".to_string(),
                coordinate: None,
            }),
        };

        let data = packet.get_data();
        assert!(data.is_some());
        let data = data.unwrap();

        let raw_data: Result<AudioFramePacket, ()> = data.to_owned().try_into();
        assert!(raw_data.is_ok());
        let raw_data = raw_data.unwrap();
        assert!(raw_data.length == 240);
        assert!(raw_data.sample_rate == 48000);
    }
}

/// A single, Opus encoded audio frame
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AudioFramePacket {
    pub length: usize,
    pub sample_rate: u32,
    pub data: Vec<u8>,
    pub author: String,
    pub coordinate: Option<Coordinate>,
}

impl TryFrom<QuicNetworkPacketData> for AudioFramePacket {
    type Error = ();

    fn try_from(value: QuicNetworkPacketData) -> Result<Self, Self::Error> {
        match value {
            QuicNetworkPacketData::AudioFrame(c) => Ok(c),
            _ => Err(()),
        }
    }
}

/// General Player Positioning data
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PlayerDataPacket {
    pub players: Vec<crate::Player>,
}

impl TryFrom<QuicNetworkPacketData> for PlayerDataPacket {
    type Error = ();

    fn try_from(value: QuicNetworkPacketData) -> Result<Self, Self::Error> {
        match value {
            QuicNetworkPacketData::PlayerData(c) => Ok(c),
            _ => Err(()),
        }
    }
}

/// Debug Packet
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DebugPacket(pub String);

impl TryFrom<QuicNetworkPacketData> for DebugPacket {
    type Error = ();

    fn try_from(value: QuicNetworkPacketData) -> Result<Self, Self::Error> {
        match value {
            QuicNetworkPacketData::Debug(c) => Ok(c),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChannelEventPacket {
    pub event: ChannelEvents,
    pub name: String,
    pub channel: String,
}

impl TryFrom<QuicNetworkPacketData> for ChannelEventPacket {
    type Error = ();

    fn try_from(value: QuicNetworkPacketData) -> Result<Self, Self::Error> {
        match value {
            QuicNetworkPacketData::ChannelEvent(c) => Ok(c),
            _ => Err(()),
        }
    }
}
