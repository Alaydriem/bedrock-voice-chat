use bytes::Bytes;

pub enum RoutedPacket {
    Serialized(Bytes),
}
