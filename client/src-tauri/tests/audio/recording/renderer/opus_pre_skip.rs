// `Mp4Renderer::create_opus_box` writes a fixed `pre_skip` of 312 into every DopsBox. For a
// passthrough that describes the recorder's encoder; for a spatial or jukebox track it describes
// the encoder in `MixedOpusStream`, which is a different encoder entirely.
//
// A wrong pre_skip offsets the file against its own timecode track by a few milliseconds, which
// an NLE user would read as BVC being out of sync. This pins the two together so a libopus or
// bitrate change cannot silently separate them.
const DECLARED_PRE_SKIP: i32 = 312;

fn lookahead_for(channels: opus2::Channels) -> i32 {
    let mut encoder = opus2::Encoder::new(48000, channels, opus2::Application::Audio)
        .expect("an encoder at the rate every recording uses");
    encoder
        .set_bitrate(opus2::Bitrate::Bits(64_000))
        .expect("the bitrate the mixed stream encodes at");

    encoder.get_lookahead().expect("libopus reports a lookahead")
}

#[test]
fn the_declared_pre_skip_matches_what_a_stereo_re_encode_actually_delays() {
    assert_eq!(lookahead_for(opus2::Channels::Stereo), DECLARED_PRE_SKIP);
}

#[test]
fn a_mono_re_encode_delays_by_the_same_amount() {
    assert_eq!(lookahead_for(opus2::Channels::Mono), DECLARED_PRE_SKIP);
}
