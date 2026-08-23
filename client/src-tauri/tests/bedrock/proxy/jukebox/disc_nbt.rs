use bytes::Bytes;

use bvc_client_lib::bedrock::DiscNbt;

#[test]
fn extracts_valid_uuid_from_blob() {
    let uuid = "0192f0d8-7a2a-7c01-9b41-2c7d3a8e9f01";
    let mut blob = vec![0u8, 1, 2, 3, 0xaa, 0xbb];
    blob.extend_from_slice(b"BVC: ");
    blob.extend_from_slice(uuid.as_bytes());
    blob.extend_from_slice(&[0xcc, 0xdd]);
    let extracted = DiscNbt::extract_audio_id(&Bytes::from(blob));
    assert_eq!(extracted.as_deref(), Some(uuid));
}

#[test]
fn returns_none_when_marker_missing() {
    let blob = Bytes::from_static(b"some other NBT-ish bytes without the marker");
    assert!(DiscNbt::extract_audio_id(&blob).is_none());
}

#[test]
fn returns_none_for_invalid_uuid() {
    let mut blob = Vec::new();
    blob.extend_from_slice(b"BVC: ");
    blob.extend_from_slice(b"not-a-real-uuid-string-of-the-right-length");
    assert!(DiscNbt::extract_audio_id(&Bytes::from(blob)).is_none());
}

#[test]
fn returns_none_for_too_short_blob() {
    let blob = Bytes::from_static(b"BVC: short");
    assert!(DiscNbt::extract_audio_id(&blob).is_none());
}
