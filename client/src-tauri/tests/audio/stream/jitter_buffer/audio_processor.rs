use std::f32::consts::PI;

use bvc_client_lib::AudioProcessor;

const RATE: u32 = 48_000;
const FRAME: usize = 960;
const CAPACITY_FRAMES: usize = 6;

// A packet libopus refuses before touching decoder state: code 3 with a frame count of zero.
const INVALID_PACKET: [u8; 2] = [0x03, 0x00];

// The largest step between adjacent samples of a 0.5-amplitude tone near 440 Hz is about 0.029. A
// step several times that is a discontinuity: an audible click.
const MAX_STEP: f32 = 0.08;

// Five frames of one continuous tone, encoded the way a speaker would.
fn frames() -> Vec<Vec<u8>> {
    tone_frames(5, 440.0)
}

fn tone_frames(count: usize, hz: f32) -> Vec<Vec<u8>> {
    let mut encoder = opus2::Encoder::new(RATE, opus2::Channels::Mono, opus2::Application::Voip)
        .expect("encoder");
    encoder
        .set_bitrate(opus2::Bitrate::Bits(64_000))
        .expect("bitrate");
    (0..count)
        .map(|frame| {
            let pcm: Vec<f32> = (0..FRAME)
                .map(|n| {
                    let t = (frame * FRAME + n) as f32 / RATE as f32;
                    0.5 * (2.0 * PI * hz * t).sin()
                })
                .collect();
            encoder.encode_vec_float(&pcm, 4_000).expect("encode")
        })
        .collect()
}

fn largest_step(samples: &[f32]) -> (usize, f32) {
    samples
        .windows(2)
        .enumerate()
        .map(|(i, w)| (i, (w[1] - w[0]).abs()))
        .fold((0, 0.0), |best, cur| if cur.1 > best.1 { cur } else { best })
}

fn drain_frame(processor: &mut AudioProcessor) -> Vec<f32> {
    (0..FRAME)
        .map(|_| processor.next_sample().expect("a full frame is queued"))
        .collect()
}

// One call to `generate_plc` must stand for one lost 20 ms frame. The reference decoder
// conceals exactly one frame between the same two real frames. If the processor's concealment
// runs longer, its decoder state diverges and the frame after the gap no longer matches.
#[test]
fn one_concealment_call_advances_the_decoder_by_one_frame() {
    let frames = frames();

    let mut processor = AudioProcessor::new(RATE, CAPACITY_FRAMES).expect("processor");
    processor.decode_opus(&frames[0]).expect("frame 0");
    drain_frame(&mut processor);
    processor.generate_plc().expect("conceal frame 1");
    drain_frame(&mut processor);
    processor.decode_opus(&frames[2]).expect("frame 2");
    let after_gap = drain_frame(&mut processor);

    let mut reference = opus2::Decoder::new(RATE, opus2::Channels::Mono).expect("decoder");
    let mut scratch = vec![0.0f32; FRAME];
    reference
        .decode_float(&frames[0], &mut scratch, false)
        .expect("ref 0");
    // An empty packet is concealment; libopus ignores the FEC flag without data, so pass false.
    reference
        .decode_float(&[], &mut scratch[..FRAME], false)
        .expect("ref conceal");
    let mut expected = vec![0.0f32; FRAME];
    reference
        .decode_float(&frames[2], &mut expected, false)
        .expect("ref 2");

    let worst = after_gap
        .iter()
        .zip(&expected)
        .map(|(a, b)| (a - b).abs())
        .fold(0.0f32, f32::max);

    assert!(
        worst < 1e-6,
        "the frame after one concealed frame diverges from a one-frame reference by {worst}; \
         concealment is running longer than one frame"
    );
}

// The decoder is reset after ten *consecutive* decode errors. A good frame between two runs of
// errors ends the run; resetting anyway throws away decoder state mid-speech, and the next frame
// no longer matches a decoder that never reset.
#[test]
fn a_good_frame_ends_a_run_of_decode_errors() {
    let frames = tone_frames(3, 440.0);

    let mut processor = AudioProcessor::new(RATE, CAPACITY_FRAMES).expect("processor");
    processor.decode_opus(&frames[0]).expect("frame 0");
    drain_frame(&mut processor);
    for _ in 0..9 {
        assert!(processor.decode_opus(&INVALID_PACKET).is_err());
    }
    processor.decode_opus(&frames[1]).expect("frame 1");
    drain_frame(&mut processor);
    for _ in 0..9 {
        assert!(processor.decode_opus(&INVALID_PACKET).is_err());
    }
    processor.decode_opus(&frames[2]).expect("frame 2");
    let actual = drain_frame(&mut processor);

    let mut reference = opus2::Decoder::new(RATE, opus2::Channels::Mono).expect("decoder");
    let mut expected = vec![0.0f32; FRAME];
    for frame in &frames {
        reference
            .decode_float(frame, &mut expected, false)
            .expect("reference");
    }

    let worst = actual
        .iter()
        .zip(&expected)
        .map(|(a, b)| (a - b).abs())
        .fold(0.0f32, f32::max);

    assert!(
        worst < 1e-6,
        "the frame after two separate runs of nine errors diverges from a decoder that never \
         reset by {worst}; the error count is not cleared by a good frame"
    );
}

// Shedding a frame joins the frame before it to the frame after it. Without a crossfade the join
// is a step in the waveform: a click on every shed. At 425 Hz a frame is 8.5 cycles, so a shed
// shifts the phase by half a cycle and the uncrossfaded join is near its worst.
#[test]
fn shedding_a_frame_leaves_no_discontinuity() {
    const SHED: [usize; 3] = [4, 8, 12];
    let frames = tone_frames(16, 425.0);

    let mut processor = AudioProcessor::new(RATE, CAPACITY_FRAMES).expect("processor");
    let mut output = Vec::new();
    for (index, frame) in frames.iter().enumerate() {
        if SHED.contains(&index) {
            processor.decode_held(frame).expect("shed candidate");
            processor.discard_held();
            continue;
        }
        processor.decode_opus(frame).expect("played frame");
        output.extend(drain_frame(&mut processor));
    }

    // The first frame carries the decoder's own onset; judge from the second frame on.
    let (at, step) = largest_step(&output[FRAME..]);
    assert!(
        step < MAX_STEP,
        "a step of {step} at sample {}; the joins are at samples 3840, 6720 and 9600. \
         The shed frame was cut, not crossfaded",
        at + FRAME
    );
}

// Concealment runs five frames and then falls to silence. Neither the fall to silence nor the
// return of real audio may be a step.
#[test]
fn concealment_fades_to_silence_and_back() {
    let frames = tone_frames(6, 440.0);

    let mut processor = AudioProcessor::new(RATE, CAPACITY_FRAMES).expect("processor");
    let mut output = Vec::new();
    for frame in &frames[..3] {
        processor.decode_opus(frame).expect("lead-in");
        output.extend(drain_frame(&mut processor));
    }
    for _ in 0..8 {
        processor.generate_plc().expect("conceal");
        output.extend(drain_frame(&mut processor));
    }
    processor.reset_plc_counter();
    for frame in &frames[3..] {
        processor.decode_opus(frame).expect("after concealment");
        output.extend(drain_frame(&mut processor));
    }

    let (at, step) = largest_step(&output[FRAME..]);
    assert!(
        step < MAX_STEP,
        "a step of {step} at sample {}; concealment ends at {} and audio returns at {}",
        at + FRAME,
        8 * FRAME,
        11 * FRAME
    );
}
