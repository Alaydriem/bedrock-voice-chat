use common::structs::audio::StreamConfig;

fn config(sample_rate: u32, sample_format: &str) -> StreamConfig {
    StreamConfig {
        channels: 1,
        sample_rate,
        sample_format: String::from(sample_format),
        buffer_size_min: 0,
        buffer_size_max: 0,
    }
}

fn formats(configs: &[StreamConfig]) -> Vec<&str> {
    configs
        .iter()
        .map(|c| c.sample_format.as_str())
        .collect()
}

fn rates(configs: &[StreamConfig]) -> Vec<u32> {
    configs.iter().map(|c| c.sample_rate).collect()
}

#[test]
fn the_highest_rate_leads() {
    let ordered = StreamConfig::preference_order(vec![
        config(44100, "f32"),
        config(48000, "f32"),
        config(8000, "f32"),
    ]);

    assert_eq!(rates(&ordered), vec![48000, 44100, 8000]);
}

/// The capture path takes the first entry, so an f64 config placed anywhere above a
/// format the hardware shares with the pipeline would be selected in its place.
#[test]
fn f64_ranks_below_every_other_format_at_a_higher_rate() {
    let ordered = StreamConfig::preference_order(vec![
        config(96000, "f64"),
        config(48000, "f32"),
    ]);

    assert_eq!(formats(&ordered), vec!["f32", "f64"]);
}

#[test]
fn f64_ranks_below_every_other_format_at_the_same_rate() {
    let ordered = StreamConfig::preference_order(vec![
        config(48000, "f64"),
        config(48000, "i16"),
    ]);

    assert_eq!(formats(&ordered), vec!["i16", "f64"]);
}

/// A device offering nothing else must still resolve to a config rather than to the
/// "does not have any supported stream configs" error.
#[test]
fn f64_survives_when_it_is_the_only_format_offered() {
    let ordered = StreamConfig::preference_order(vec![
        config(44100, "f64"),
        config(48000, "f64"),
    ]);

    assert_eq!(rates(&ordered), vec![48000, 44100]);
}

/// Nothing ranks the accepted formats against each other, so the enumeration order the
/// backend reported is what separates them at one rate.
#[test]
fn the_accepted_formats_keep_the_order_they_arrived_in() {
    let ordered = StreamConfig::preference_order(vec![
        config(48000, "i16"),
        config(48000, "f32"),
        config(48000, "i32"),
    ]);

    assert_eq!(formats(&ordered), vec!["i16", "f32", "i32"]);
}
