use bvc_client_lib::audio::recording::renderer::mp4::timecode::{Timecode, TimecodeSample};

#[test]
fn a_sample_encodes_to_four_bytes() {
    let tc = Timecode::new(1705329045500, 48000);
    let sample = TimecodeSample::from_timecode(tc);
    let bytes = sample.to_bytes();
    assert_eq!(bytes.len(), 4);
}

#[test]
fn the_vec_and_array_encodings_agree() {
    let tc = Timecode::new(1705329045500, 48000);
    let sample = TimecodeSample::from_timecode(tc);
    let vec = sample.to_vec();
    let bytes = sample.to_bytes();
    assert_eq!(vec.as_slice(), bytes.as_slice());
}

#[test]
fn the_frame_number_is_encoded_big_endian() {
    let tc = Timecode::new(0, 48000);
    let sample = TimecodeSample::from_timecode(tc);
    let frame_num = tc.to_frame_number();
    let bytes = sample.to_bytes();

    let decoded = u32::from_be_bytes(bytes);
    assert_eq!(decoded, frame_num);
}
