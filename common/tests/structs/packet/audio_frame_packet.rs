use common::Coordinate;
use common::game_data::Dimension;
use common::structs::packet::{AudioFrameMetadata, AudioFramePacket, JukeboxMetadata};

fn jukebox() -> AudioFrameMetadata {
    AudioFrameMetadata::Jukebox(JukeboxMetadata::new(
        Coordinate {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        },
        "playback-1".to_string(),
        Dimension::Overworld,
    ))
}

// A sealed frame reaches only group members, and the server must never be able to place it in
// space: the speaker and spatial fields are what proximity routing reads.
#[test]
fn into_sealed_strips_speaker_and_spatial() {
    let packet = AudioFramePacket::new(vec![1, 2, 3], None, Some(true)).with_metadata(vec![jukebox()]);
    assert!(!packet.is_sealed());

    let sealed = packet.into_sealed(vec![9, 9, 9, 9]);

    assert!(sealed.is_sealed());
    assert!(sealed.speaker.is_none());
    assert!(sealed.spatial.is_none());
    assert_eq!(sealed.data.as_ref(), &[9, 9, 9, 9]);
    assert!(sealed.metadata.contains(&jukebox()));
}
